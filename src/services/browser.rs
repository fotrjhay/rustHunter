use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use thirtyfour::error::{WebDriverErrorInner, WebDriverResult};
use thirtyfour::prelude::*;
use thirtyfour::ChromeCapabilities;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::JobListing;
use crate::utils::captcha::CAPTCHA_SCREENSHOT_FILE;
use crate::utils::delay::human_delay;

#[derive(Debug)]
pub struct BrowserService {
    config: AppConfig,
}

impl BrowserService {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub async fn navigate_and_extract(
        &self,
        url: &str,
        include_description: bool,
    ) -> Result<Vec<JobListing>, AppError> {
        info!(url, include_description, "opening search page in browser");

        let driver = self.create_driver().await?;
        let navigation = async {
            driver
                .goto(url)
                .await
                .map_err(|err| AppError::Browser(err.to_string()))?;
            let current_url = driver
                .current_url()
                .await
                .map_err(|err| AppError::Browser(err.to_string()))?;
            info!(%current_url, "browser navigation completed");
            self.wait_for_search_page_state(&driver).await?;

            if self.detect_captcha(&driver).await? {
                self.capture_captcha_screenshot(&driver).await;
                return Err(AppError::CaptchaDetected);
            }

            self.extract_search_results(&driver)
                .await
                .map_err(|err| AppError::Browser(err.to_string()))
        };

        let result = timeout(Duration::from_millis(self.config.page_timeout), navigation).await;

        let result = match result {
            Ok(Ok(mut results)) => {
                if include_description {
                    self.populate_descriptions(&driver, &mut results).await;
                }

                Ok(results)
            }
            Ok(Err(err)) => Err(err),
            Err(_) => Err(AppError::Timeout),
        };

        if let Err(err) = driver.quit().await {
            warn!(error = %err, "failed to close browser session");
        }

        result
    }

    pub async fn apply_to_vacancy(
        &self,
        vacancy_url: &str,
        cover_letter: &str,
        resume_id: Option<&str>,
    ) -> Result<(), AppError> {
        info!(%vacancy_url, has_resume_id = resume_id.is_some(), "starting browser apply review flow");

        let driver = self.create_visible_driver().await?;
        let apply_flow = async {
            driver
                .goto(vacancy_url)
                .await
                .map_err(|err| AppError::Browser(err.to_string()))?;
            match self.wait_for_vacancy_page_state(&driver).await {
                Ok(()) => {}
                Err(AppError::Timeout) => {
                    warn!("timed out waiting for vacancy apply button; continuing manual review with browser open");
                    self.capture_debug_screenshot(&driver, "apply_error.png")
                        .await;
                }
                Err(err) => return Err(err),
            }

            if self.detect_captcha(&driver).await? {
                self.capture_captcha_screenshot(&driver).await;
                return Err(AppError::CaptchaDetected);
            }

            human_delay().await;
            self.capture_debug_screenshot(&driver, "apply_before_click.png")
                .await;

            info!("clicking vacancy apply/respond button");
            if let Err(err) = click_first_interactable(
                &driver,
                &[
                    Locator::Css("[data-qa='vacancy-response-link-top']"),
                    Locator::Css("[data-qa='vacancy-response-link-bottom']"),
                    Locator::Css("[data-qa='vacancy-response-link']"),
                    Locator::Css("a[href*='vacancy_response']"),
                    Locator::XPath("//button[contains(translate(normalize-space(.), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'apply')]"),
                    Locator::XPath("//a[contains(translate(normalize-space(.), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'apply')]"),
                    Locator::XPath("//button[contains(translate(normalize-space(.), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'respond')]"),
                    Locator::XPath("//a[contains(translate(normalize-space(.), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'respond')]"),
                ],
            )
            .await
            {
                warn!(
                    error = %err,
                    "failed to click apply/respond button; continuing manual review with browser open"
                );
                self.capture_debug_screenshot(&driver, "apply_error.png")
                    .await;
            }

            match self.wait_for_apply_form(&driver).await {
                Ok(()) => {
                    self.capture_debug_screenshot(&driver, "apply_modal.png")
                        .await;
                }
                Err(AppError::Timeout) => {
                    warn!("timed out waiting for apply form; continuing manual review with browser open");
                    self.capture_debug_screenshot(&driver, "apply_error.png")
                        .await;
                }
                Err(err) => return Err(err),
            }

            if self.detect_captcha(&driver).await? {
                self.capture_captcha_screenshot(&driver).await;
                return Err(AppError::CaptchaDetected);
            }

            if let Some(resume_id) = resume_id {
                self.select_resume_if_available(&driver, resume_id).await;
            }

            self.fill_cover_letter_if_available(&driver, cover_letter)
                .await;

            self.capture_debug_screenshot(&driver, "apply_ready_for_review.png")
                .await;
            info!("application prepared for manual review; browser remains open and no submit button was clicked");
            info!("Review application manually and close the browser window when done.");

            Ok(())
        };

        match apply_flow.await {
            Ok(()) => {
                info!("waiting for manual review; close the browser window to complete /apply");
                self.wait_for_manual_browser_close(&driver).await;
                info!("browser closed by user");
                info!("success returned to n8n");
                Ok(())
            }
            Err(err) if is_manual_browser_close_error(&err) => {
                info!(
                    error = %err,
                    "browser closed by user during apply review flow"
                );
                info!("success returned to n8n");
                Ok(())
            }
            Err(err) => {
                self.capture_debug_screenshot(&driver, "apply_error.png")
                    .await;
                Err(err)
            }
        }
    }

    pub async fn manual_login(&self) -> Result<(), AppError> {
        info!("opening browser for manual HH.ru login");

        let driver = self.create_driver().await?;
        let result = async {
            driver
                .goto(format!(
                    "https://hh.ru/account/login?locale={}",
                    urlencoding::encode(&self.config.hh_locale)
                ))
                .await
                .map_err(|err| AppError::Browser(err.to_string()))?;

            info!("login page opened; complete login in Chrome, then press Ctrl+C here");
            tokio::signal::ctrl_c()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;

            Ok::<(), AppError>(())
        }
        .await;

        if let Err(err) = driver.quit().await {
            warn!(error = %err, "failed to close browser session");
        }

        result
    }

    async fn create_driver(&self) -> Result<WebDriver, AppError> {
        self.create_driver_with_headless(self.config.browser_headless)
            .await
    }

    async fn create_visible_driver(&self) -> Result<WebDriver, AppError> {
        self.create_driver_with_headless(false).await
    }

    async fn create_driver_with_headless(&self, headless: bool) -> Result<WebDriver, AppError> {
        self.check_driver_available().await?;

        let caps = self.chrome_capabilities(headless)?;

        debug!(
            driver_url = %self.config.browser_driver_url,
            headless,
            profile_dir = %self.config.browser_profile_dir,
            "creating webdriver session"
        );

        WebDriver::new(&self.config.browser_driver_url, caps)
            .await
            .map_err(|err| self.classify_driver_error(err))
    }

    fn classify_driver_error(&self, err: WebDriverError) -> AppError {
        let message = err.to_string();
        let message_lower = message.to_ascii_lowercase();

        if message_lower.contains("user data directory is already in use")
            || message_lower.contains("profile appears to be in use")
            || (message_lower.contains("chrome failed to start")
                && message_lower.contains("devtoolsactiveport"))
        {
            return AppError::ProfileInUse(self.config.browser_profile_dir.clone());
        }

        AppError::Browser(message)
    }

    async fn check_driver_available(&self) -> Result<(), AppError> {
        let status_url = format!(
            "{}/status",
            self.config.browser_driver_url.trim_end_matches('/')
        );

        match reqwest::get(&status_url).await {
            Ok(response) if response.status().is_success() => Ok(()),
            Ok(response) => Err(AppError::DriverUnavailable(format!(
                "{} returned HTTP {}",
                self.config.browser_driver_url,
                response.status()
            ))),
            Err(_) => Err(AppError::DriverUnavailable(
                self.config.browser_driver_url.clone(),
            )),
        }
    }

    fn chrome_capabilities(&self, headless: bool) -> Result<ChromeCapabilities, AppError> {
        let mut caps = DesiredCapabilities::chrome();
        let profile_dir = self.profile_dir()?;

        add_chrome_arg(
            &mut caps,
            &format!("--user-data-dir={}", profile_dir.to_string_lossy()),
        )?;
        add_chrome_arg(&mut caps, "--disable-gpu")?;
        add_chrome_arg(&mut caps, "--disable-dev-shm-usage")?;
        add_chrome_arg(&mut caps, "--disable-background-networking")?;
        add_chrome_arg(&mut caps, "--disable-client-side-phishing-detection")?;
        add_chrome_arg(&mut caps, "--disable-component-update")?;
        add_chrome_arg(&mut caps, "--disable-default-apps")?;
        add_chrome_arg(&mut caps, "--disable-domain-reliability")?;
        add_chrome_arg(&mut caps, "--disable-extensions")?;
        add_chrome_arg(&mut caps, "--disable-logging")?;
        add_chrome_arg(&mut caps, "--disable-notifications")?;
        add_chrome_arg(&mut caps, "--disable-sync")?;
        add_chrome_arg(&mut caps, "--no-first-run")?;
        add_chrome_arg(&mut caps, "--no-default-browser-check")?;
        add_chrome_arg(&mut caps, "--no-pings")?;
        add_chrome_arg(&mut caps, "--log-level=3")?;
        add_chrome_arg(&mut caps, "--silent")?;
        add_chrome_arg(&mut caps, "--v=0")?;
        add_chrome_arg(&mut caps, "--remote-debugging-port=0")?;
        add_chrome_arg(&mut caps, "--safebrowsing-disable-auto-update")?;
        add_chrome_arg(
            &mut caps,
            "--disable-features=AutofillServerCommunication,CalculateNativeWinOcclusion,CertificateTransparencyComponentUpdater,GlobalMediaControls,InterestFeedContentSuggestions,MediaRouter,NotificationTriggers,OptimizationHints,PushMessaging",
        )?;
        add_chrome_arg(&mut caps, "--window-size=1365,900")?;
        add_chrome_experimental_option(&mut caps, "excludeSwitches", vec!["enable-logging"])?;
        add_chrome_experimental_option(&mut caps, "useAutomationExtension", false)?;
        add_chrome_experimental_option(
            &mut caps,
            "prefs",
            serde_json::json!({
                "intl.accept_languages": "en-US,en",
                "translate": {
                    "enabled": true
                }
            }),
        )?;

        if headless {
            add_chrome_arg(&mut caps, "--headless=new")?;
        }

        Ok(caps)
    }

    fn profile_dir(&self) -> Result<PathBuf, AppError> {
        let configured = PathBuf::from(&self.config.browser_profile_dir);
        let path = if configured.is_absolute() {
            configured
        } else {
            std::env::current_dir()
                .map_err(|err| AppError::Internal(err.to_string()))?
                .join(configured)
        };

        fs::create_dir_all(&path).map_err(|err| {
            AppError::Internal(format!(
                "failed to create browser profile directory {}: {err}",
                path.display()
            ))
        })?;

        Ok(path)
    }

    async fn extract_search_results(&self, driver: &WebDriver) -> WebDriverResult<Vec<JobListing>> {
        let cards = find_all_by_selectors(
            driver,
            &[
                "[data-qa='vacancy-serp__vacancy']",
                "[data-qa='serp-item']",
                ".vacancy-serp-item",
            ],
        )
        .await?;

        let mut listings = Vec::new();

        for card in cards {
            match extract_listing(&card).await {
                Ok(Some(listing)) => listings.push(listing),
                Ok(None) => debug!("skipping incomplete search result"),
                Err(err) => warn!(error = %err, "failed to parse search result"),
            }
        }

        info!(count = listings.len(), "extracted search results");
        Ok(listings)
    }

    async fn populate_descriptions(&self, driver: &WebDriver, listings: &mut [JobListing]) {
        info!(
            count = listings.len(),
            "fetching vacancy descriptions from detail pages"
        );

        for listing in listings {
            human_delay().await;

            let vacancy_url = listing.url.clone();
            let description_flow = async {
                info!(url = %vacancy_url, "opening vacancy page for description");
                driver
                    .goto(&vacancy_url)
                    .await
                    .map_err(|err| AppError::Browser(err.to_string()))?;

                self.wait_for_vacancy_description_state(driver).await?;

                if self.detect_captcha(driver).await? {
                    self.capture_captcha_screenshot(driver).await;
                    return Err(AppError::CaptchaDetected);
                }

                self.extract_vacancy_description(driver).await
            };

            match timeout(
                Duration::from_millis(self.config.page_timeout),
                description_flow,
            )
            .await
            {
                Ok(Ok(description)) => {
                    listing.description = description;
                }
                Ok(Err(err)) => {
                    warn!(
                        url = %vacancy_url,
                        error = %err,
                        "skipping vacancy description after page error"
                    );
                    if matches!(err, AppError::Timeout) {
                        self.capture_debug_screenshot(driver, "description_timeout.png")
                            .await;
                    }
                }
                Err(_) => {
                    warn!(
                        url = %vacancy_url,
                        "skipping vacancy description after timeout"
                    );
                    self.capture_debug_screenshot(driver, "description_timeout.png")
                        .await;
                }
            }
        }
    }

    async fn wait_for_search_page_state(&self, driver: &WebDriver) -> Result<(), AppError> {
        let deadline = Instant::now() + Duration::from_millis(self.config.page_timeout);

        while Instant::now() < deadline {
            if self.detect_captcha(driver).await? {
                return Ok(());
            }

            let has_results = has_any_css(
                driver,
                &[
                    "[data-qa='vacancy-serp__vacancy']",
                    "[data-qa='serp-item']",
                    ".vacancy-serp-item",
                ],
            )
            .await?;

            if has_results {
                return Ok(());
            }

            let has_empty_state = any_displayed_xpath(
                driver,
                &[
                    "//*[contains(translate(normalize-space(.), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'ничего не найдено')]",
                    "//*[contains(translate(normalize-space(.), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'no results')]",
                ],
            )
            .await?;

            if has_empty_state {
                info!("search page loaded with empty result state");
                return Ok(());
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err(AppError::Timeout)
    }

    async fn detect_captcha(&self, driver: &WebDriver) -> Result<bool, AppError> {
        let captcha_frame = any_displayed_css(
            driver,
            &[
                "iframe[src*='captcha']",
                "iframe[name*='captcha']",
                "iframe[id*='captcha']",
                "iframe[title*='captcha']",
            ],
        )
        .await?;

        if captcha_frame {
            warn!("captcha iframe detected");
            return Ok(true);
        }

        let captcha_input = any_displayed_css(
            driver,
            &[
                "input[name*='captcha']",
                "input[id*='captcha']",
                "input[class*='captcha']",
                "input[autocomplete='one-time-code']",
            ],
        )
        .await?;

        if captcha_input {
            warn!("captcha input detected");
            return Ok(true);
        }

        let robot_indicator = any_displayed_xpath(
            driver,
            &[
                "//*[contains(translate(normalize-space(.), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'i am not a robot')]",
                "//*[contains(translate(normalize-space(.), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), 'not a robot')]",
            ],
        )
        .await?;

        if robot_indicator {
            warn!("visible robot-check indicator detected");
            return Ok(true);
        }

        Ok(false)
    }

    async fn capture_captcha_screenshot(&self, driver: &WebDriver) {
        let path = match self.screenshot_path(CAPTCHA_SCREENSHOT_FILE) {
            Ok(path) => path,
            Err(err) => {
                warn!(error = %err, "failed to prepare captcha screenshot path");
                return;
            }
        };

        match driver.screenshot(&path).await {
            Ok(()) => warn!(path = %path.display(), "captcha screenshot saved"),
            Err(err) => {
                warn!(error = %err, path = %path.display(), "failed to save captcha screenshot")
            }
        }
    }

    async fn capture_debug_screenshot(&self, driver: &WebDriver, screenshot_path: &str) {
        let path = match self.screenshot_path(screenshot_path) {
            Ok(path) => path,
            Err(err) => {
                warn!(error = %err, file = screenshot_path, "failed to prepare debug screenshot path");
                return;
            }
        };

        match driver.screenshot(&path).await {
            Ok(()) => info!(path = %path.display(), "debug screenshot saved"),
            Err(err) => {
                warn!(error = %err, path = %path.display(), "failed to save debug screenshot")
            }
        }
    }

    fn screenshot_path(&self, file_name: &str) -> Result<PathBuf, AppError> {
        let dir = self.profile_dir()?.join("screenshots");
        fs::create_dir_all(&dir).map_err(|err| {
            AppError::Internal(format!(
                "failed to create screenshot directory {}: {err}",
                dir.display()
            ))
        })?;

        Ok(dir.join(file_name))
    }

    async fn wait_for_manual_browser_close(&self, driver: &WebDriver) {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            match driver.windows().await {
                Ok(handles) if handles.is_empty() => return,
                Ok(_) => {}
                Err(err) => {
                    debug!(error = %err, "webdriver window-handle check failed; treating browser as closed");
                    return;
                }
            }

            if let Err(err) = driver.current_url().await {
                debug!(error = %err, "webdriver current-url check failed; treating browser as closed");
                return;
            }
        }
    }

    async fn wait_for_vacancy_page_state(&self, driver: &WebDriver) -> Result<(), AppError> {
        let deadline = Instant::now() + Duration::from_millis(self.config.page_timeout);

        while Instant::now() < deadline {
            if self.detect_captcha(driver).await? {
                return Ok(());
            }

            let has_apply_button = has_any_locator(
                driver,
                &[
                    Locator::Css("[data-qa='vacancy-response-link-top']"),
                    Locator::Css("[data-qa='vacancy-response-link-bottom']"),
                    Locator::Css("[data-qa='vacancy-response-link']"),
                    Locator::Css("a[href*='vacancy_response']"),
                ],
            )
            .await?;

            if has_apply_button {
                return Ok(());
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err(AppError::Timeout)
    }

    async fn wait_for_vacancy_description_state(&self, driver: &WebDriver) -> Result<(), AppError> {
        let deadline = Instant::now() + Duration::from_millis(self.config.page_timeout);

        while Instant::now() < deadline {
            if self.detect_captcha(driver).await? {
                return Ok(());
            }

            let has_description = has_any_locator(
                driver,
                &[
                    Locator::Css("[data-qa='vacancy-description']"),
                    Locator::Css("[data-qa='vacancy-description-text']"),
                    Locator::Css(".vacancy-description"),
                    Locator::Css("[data-qa='vacancy-title']"),
                ],
            )
            .await?;

            if has_description {
                return Ok(());
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err(AppError::Timeout)
    }

    async fn extract_vacancy_description(&self, driver: &WebDriver) -> Result<String, AppError> {
        match find_first_available(
            driver,
            &[
                Locator::Css("[data-qa='vacancy-description']"),
                Locator::Css("[data-qa='vacancy-description-text']"),
                Locator::Css(".vacancy-description"),
            ],
        )
        .await?
        {
            Some(element) => element
                .text()
                .await
                .map(clean_text)
                .map_err(|err| AppError::Browser(err.to_string())),
            None => {
                info!("vacancy description element not available; leaving description empty");
                Ok(String::new())
            }
        }
    }

    async fn wait_for_apply_form(&self, driver: &WebDriver) -> Result<(), AppError> {
        let deadline = Instant::now() + Duration::from_millis(self.config.page_timeout);

        while Instant::now() < deadline {
            if self.detect_captcha(driver).await? {
                return Ok(());
            }

            let has_form = has_any_locator(
                driver,
                &[
                    Locator::Css("[data-qa='vacancy-response-popup']"),
                    Locator::Css("[data-qa='vacancy-response-form']"),
                    Locator::Css("[data-qa='vacancy-response-popup-form-letter-input']"),
                    Locator::Css("[data-qa='vacancy-response-submit-popup']"),
                    Locator::Css("[data-qa='vacancy-response-submit']"),
                    Locator::Css("textarea"),
                ],
            )
            .await?;

            if has_form {
                return Ok(());
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err(AppError::Timeout)
    }

    async fn fill_cover_letter_if_available(&self, driver: &WebDriver, cover_letter: &str) {
        match find_first_interactable(
            driver,
            &[
                Locator::Css("[data-qa='vacancy-response-popup-form-letter-input']"),
                Locator::Css("[data-qa='vacancy-response-letter-input']"),
                Locator::Css("textarea[name='letter']"),
                Locator::Css("textarea"),
            ],
        )
        .await
        {
            Ok(Some(textarea)) => {
                info!("filling cover letter textarea");
                if let Err(err) = textarea.clear().await {
                    debug!(error = %err, "failed to clear cover letter textarea before typing");
                }
                match textarea.send_keys(cover_letter).await {
                    Ok(()) => {}
                    Err(err) if is_non_interactable_webdriver_error(&err) => {
                        warn!(
                            error = %err,
                            "cover letter textarea was not interactable; continuing manual review without auto-fill"
                        );
                    }
                    Err(err) => {
                        warn!(
                            error = %err,
                            "failed to type cover letter; continuing manual review without auto-fill"
                        );
                    }
                }
            }
            Ok(None) => {
                info!("cover letter textarea not available; continuing without filling one");
            }
            Err(err) if is_manual_browser_close_error(&err) => {
                info!(
                    error = %err,
                    "browser closed while filling cover letter; treating as manual review completion"
                );
            }
            Err(err) => {
                warn!(
                    error = %err,
                    "failed to inspect cover letter textarea; continuing manual review without auto-fill"
                );
            }
        }
    }

    async fn select_resume_if_available(&self, driver: &WebDriver, resume_id: &str) {
        let resume_id_predicate = xpath_literal(resume_id);
        let locators = vec![
            Locator::CssOwned(format!("input[value='{resume_id}']")),
            Locator::CssOwned(format!("input[data-resume-id='{resume_id}']")),
            Locator::CssOwned(format!("[data-resume-id='{resume_id}']")),
            Locator::XPathOwned(format!("//*[@value={resume_id_predicate}]")),
            Locator::XPathOwned(format!("//*[contains(@href, {resume_id_predicate})]")),
            Locator::XPathOwned(format!("//*[contains(@data-qa, {resume_id_predicate})]")),
        ];

        match find_first_interactable(driver, &locators).await {
            Ok(Some(element)) => {
                info!(resume_id, "selecting requested resume");
                match element.click().await {
                    Ok(()) => {}
                    Err(err) if is_non_interactable_webdriver_error(&err) => {
                        warn!(
                            resume_id,
                            error = %err,
                            "requested resume control was not interactable; continuing with current/default resume"
                        );
                    }
                    Err(err) => {
                        warn!(
                            resume_id,
                            error = %err,
                            "failed to select requested resume; continuing with current/default resume"
                        );
                    }
                }
            }
            Ok(None) => {
                info!(
                    resume_id,
                    "requested resume selector not found; continuing with default resume"
                );
            }
            Err(err) if is_manual_browser_close_error(&err) => {
                info!(
                    error = %err,
                    "browser closed while selecting resume; treating as manual review completion"
                );
            }
            Err(err) => {
                warn!(
                    resume_id,
                    error = %err,
                    "failed to inspect requested resume controls; continuing with current/default resume"
                );
            }
        }
    }
}

#[derive(Clone)]
enum Locator<'a> {
    Css(&'a str),
    XPath(&'a str),
    CssOwned(String),
    XPathOwned(String),
}

impl<'a> Locator<'a> {
    fn by(&'a self) -> By {
        match self {
            Self::Css(selector) => By::Css(*selector),
            Self::XPath(selector) => By::XPath(*selector),
            Self::CssOwned(selector) => By::Css(selector.as_str()),
            Self::XPathOwned(selector) => By::XPath(selector.as_str()),
        }
    }
}

fn add_chrome_arg(caps: &mut ChromeCapabilities, arg: &str) -> Result<(), AppError> {
    caps.add_arg(arg)
        .map_err(|err| AppError::Browser(err.to_string()))
}

fn add_chrome_experimental_option(
    caps: &mut ChromeCapabilities,
    name: &str,
    value: impl serde::Serialize,
) -> Result<(), AppError> {
    caps.add_experimental_option(name, value)
        .map_err(|err| AppError::Browser(err.to_string()))
}

async fn find_all_by_selectors(
    driver: &WebDriver,
    selectors: &[&str],
) -> WebDriverResult<Vec<WebElement>> {
    for selector in selectors {
        let elements = driver.find_all(By::Css(*selector)).await?;
        if !elements.is_empty() {
            debug!(selector, count = elements.len(), "matched result cards");
            return Ok(elements);
        }
    }

    Ok(Vec::new())
}

async fn find_first_available<'a>(
    driver: &WebDriver,
    locators: &'a [Locator<'a>],
) -> Result<Option<WebElement>, AppError> {
    for locator in locators {
        match driver.find(locator.by()).await {
            Ok(element) => return Ok(Some(element)),
            Err(err) if matches!(err.as_inner(), WebDriverErrorInner::NoSuchElement(_)) => continue,
            Err(err) => return Err(AppError::Browser(err.to_string())),
        }
    }

    Ok(None)
}

async fn find_first_interactable<'a>(
    driver: &WebDriver,
    locators: &'a [Locator<'a>],
) -> Result<Option<WebElement>, AppError> {
    for locator in locators {
        let elements = driver
            .find_all(locator.by())
            .await
            .map_err(|err| AppError::Browser(err.to_string()))?;

        for element in elements {
            if !is_visible_and_enabled(&element).await? {
                continue;
            }

            if let Err(err) = element.scroll_into_view().await {
                debug!(error = %err, "failed to scroll matched element into view");
            }

            return Ok(Some(element));
        }
    }

    Ok(None)
}

async fn click_first_interactable<'a>(
    driver: &WebDriver,
    locators: &'a [Locator<'a>],
) -> Result<(), String> {
    for locator in locators {
        let elements = driver
            .find_all(locator.by())
            .await
            .map_err(|err| err.to_string())?;

        for element in elements {
            match is_visible_and_enabled(&element).await {
                Ok(true) => {}
                Ok(false) => continue,
                Err(err) => {
                    debug!(error = %err, "failed to inspect matched element");
                    continue;
                }
            }

            if let Err(err) = element.scroll_into_view().await {
                debug!(error = %err, "failed to scroll matched element into view before click");
            }

            match element.click().await {
                Ok(()) => return Ok(()),
                Err(err) if is_non_interactable_webdriver_error(&err) => {
                    debug!(error = %err, "matched element could not be clicked");
                    continue;
                }
                Err(err) => return Err(err.to_string()),
            }
        }
    }

    Err("no visible enabled apply/submit button matched known selectors".to_owned())
}

async fn is_visible_and_enabled(element: &WebElement) -> Result<bool, AppError> {
    let displayed = element
        .is_displayed()
        .await
        .map_err(|err| AppError::Browser(err.to_string()))?;

    if !displayed {
        return Ok(false);
    }

    element
        .is_enabled()
        .await
        .map_err(|err| AppError::Browser(err.to_string()))
}

async fn has_any_locator<'a>(
    driver: &WebDriver,
    locators: &'a [Locator<'a>],
) -> Result<bool, AppError> {
    for locator in locators {
        let elements = driver
            .find_all(locator.by())
            .await
            .map_err(|err| AppError::Browser(err.to_string()))?;

        if !elements.is_empty() {
            return Ok(true);
        }
    }

    Ok(false)
}

async fn extract_listing(card: &WebElement) -> WebDriverResult<Option<JobListing>> {
    let Some(title_element) = find_first_child(
        card,
        &[
            "[data-qa='serp-item__title']",
            "a[data-qa='vacancy-serp__vacancy-title']",
            "a[href*='/vacancy/']",
        ],
    )
    .await?
    else {
        return Ok(None);
    };

    let title = clean_text(title_element.text().await?);
    let url = normalize_hh_url(&title_element.attr("href").await?.unwrap_or_default());

    let employer = match find_first_child(
        card,
        &[
            "[data-qa='vacancy-serp__vacancy-employer']",
            "[data-qa='vacancy-serp__vacancy-employer-text']",
            "[data-qa='serp-item__meta-info-company']",
            ".vacancy-serp-item__meta-info-company",
        ],
    )
    .await?
    {
        Some(element) => {
            let employer = clean_text(element.text().await?);
            if employer.is_empty() {
                "Unknown".to_owned()
            } else {
                employer
            }
        }
        None => "Unknown".to_owned(),
    };

    if title.is_empty() || url.is_empty() {
        return Ok(None);
    }

    Ok(Some(JobListing {
        title,
        url,
        employer,
        description: String::new(),
    }))
}

async fn find_first_child(
    parent: &WebElement,
    selectors: &[&str],
) -> WebDriverResult<Option<WebElement>> {
    for selector in selectors {
        match parent.find(By::Css(*selector)).await {
            Ok(element) => return Ok(Some(element)),
            Err(err) if matches!(err.as_inner(), WebDriverErrorInner::NoSuchElement(_)) => continue,
            Err(err) => return Err(err),
        }
    }

    Ok(None)
}

fn clean_text(value: String) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_hh_url(href: &str) -> String {
    let href = href.trim();

    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_owned();
    }

    if href.starts_with("//") {
        return format!("https:{href}");
    }

    if href.starts_with('/') {
        return format!("https://hh.ru{href}");
    }

    if href.is_empty() {
        String::new()
    } else {
        format!("https://hh.ru/{href}")
    }
}

fn xpath_literal(value: &str) -> String {
    if !value.contains('\'') {
        return format!("'{value}'");
    }

    if !value.contains('"') {
        return format!("\"{value}\"");
    }

    let parts = value
        .split('\'')
        .map(|part| format!("'{part}'"))
        .collect::<Vec<_>>()
        .join(r#", "'", "#);

    format!("concat({parts})")
}

fn is_manual_browser_close_error(err: &AppError) -> bool {
    let AppError::Browser(message) = err else {
        return false;
    };

    let message = message.to_ascii_lowercase();
    message.contains("invalid session id")
        || message.contains("session deleted")
        || message.contains("no such window")
        || message.contains("target window already closed")
        || message.contains("web view not found")
        || message.contains("disconnected")
}

fn is_non_interactable_webdriver_error(err: &WebDriverError) -> bool {
    let message = err.to_string().to_ascii_lowercase();
    message.contains("element not interactable")
        || message.contains("element click intercepted")
        || message.contains("element is not clickable")
        || message.contains("other element would receive the click")
        || message.contains("not reachable by keyboard")
}

async fn any_displayed_css(driver: &WebDriver, selectors: &[&str]) -> Result<bool, AppError> {
    for selector in selectors {
        let elements = driver
            .find_all(By::Css(*selector))
            .await
            .map_err(|err| AppError::Browser(err.to_string()))?;

        if any_displayed(elements).await? {
            debug!(selector, "matched visible anti-bot css indicator");
            return Ok(true);
        }
    }

    Ok(false)
}

async fn has_any_css(driver: &WebDriver, selectors: &[&str]) -> Result<bool, AppError> {
    for selector in selectors {
        let elements = driver
            .find_all(By::Css(*selector))
            .await
            .map_err(|err| AppError::Browser(err.to_string()))?;

        if !elements.is_empty() {
            debug!(
                selector,
                count = elements.len(),
                "matched page-state css selector"
            );
            return Ok(true);
        }
    }

    Ok(false)
}

async fn any_displayed_xpath(driver: &WebDriver, selectors: &[&str]) -> Result<bool, AppError> {
    for selector in selectors {
        let elements = driver
            .find_all(By::XPath(*selector))
            .await
            .map_err(|err| AppError::Browser(err.to_string()))?;

        if any_displayed(elements).await? {
            debug!(selector, "matched visible anti-bot xpath indicator");
            return Ok(true);
        }
    }

    Ok(false)
}

async fn any_displayed(elements: Vec<WebElement>) -> Result<bool, AppError> {
    for element in elements {
        let displayed = element
            .is_displayed()
            .await
            .map_err(|err| AppError::Browser(err.to_string()))?;

        if displayed {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{clean_text, is_manual_browser_close_error, normalize_hh_url};
    use crate::errors::AppError;

    #[test]
    fn clean_text_collapses_browser_whitespace() {
        assert_eq!(
            clean_text("  AI\n\nAutomation\tEngineer  ".to_owned()),
            "AI Automation Engineer"
        );
    }

    #[test]
    fn normalize_hh_url_keeps_absolute_urls() {
        assert_eq!(
            normalize_hh_url("https://hh.ru/vacancy/123"),
            "https://hh.ru/vacancy/123"
        );
    }

    #[test]
    fn normalize_hh_url_expands_relative_urls() {
        assert_eq!(
            normalize_hh_url("/vacancy/123"),
            "https://hh.ru/vacancy/123"
        );
    }

    #[test]
    fn invalid_session_error_is_treated_as_manual_close() {
        assert!(is_manual_browser_close_error(&AppError::Browser(
            "The WebDriver session id is invalid: invalid session id".to_owned()
        )));
    }
}
