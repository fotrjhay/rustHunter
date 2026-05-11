use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::JobListing;
use crate::services::browser::BrowserService;
use crate::utils::delay::human_delay;

#[derive(Debug)]
pub struct JobSearchService {
    config: AppConfig,
    browser: BrowserService,
}

impl JobSearchService {
    pub fn new(config: AppConfig) -> Self {
        Self {
            browser: BrowserService::new(config.clone()),
            config,
        }
    }

    pub async fn search(
        &self,
        query: &str,
        page: u32,
        include_description: bool,
    ) -> Result<Vec<JobListing>, AppError> {
        human_delay().await;

        let search_url = self.build_search_url(query, page);
        self.browser
            .navigate_and_extract(&search_url, include_description)
            .await
    }

    fn build_search_url(&self, query: &str, page: u32) -> String {
        let encoded_query = urlencoding::encode(query);
        let encoded_locale = urlencoding::encode(&self.config.hh_locale);
        let items_per_page = self.config.items_per_page.min(20);

        format!(
            "https://hh.ru/search/vacancy?text={encoded_query}&area={}&items_on_page={}&page={}&locale={encoded_locale}",
            self.config.area_code, items_per_page, page
        )
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use super::JobSearchService;
    use crate::config::AppConfig;

    #[test]
    fn build_search_url_includes_configured_locale() {
        let service = JobSearchService::new(AppConfig {
            browser_headless: false,
            browser_driver_url: "http://localhost:9515".to_owned(),
            browser_profile_dir: "browser_profile".to_owned(),
            area_code: 113,
            items_per_page: 20,
            page_timeout: 30_000,
            hh_locale: "EN".to_owned(),
            server_host: IpAddr::from([127, 0, 0, 1]),
            server_port: 3000,
        });

        let url = service.build_search_url("AI Automation Engineer", 1);

        assert_eq!(
            url,
            "https://hh.ru/search/vacancy?text=AI%20Automation%20Engineer&area=113&items_on_page=20&page=1&locale=EN"
        );
    }
}
