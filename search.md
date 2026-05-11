"""Асинхронный сервис поиска вакансий."""

import logging
from dataclasses import dataclass
from typing import Optional
from urllib.parse import quote_plus, urljoin

from playwright.async_api import Page, TimeoutError as PlaywrightTimeoutError

from ..config import get_settings
from .browser import browser_manager

logger = logging.getLogger(__name__)


@dataclass
class Vacancy:
    """Модель данных вакансии."""

    title: str
    url: str
    employer: str
    description: str = ""

    def to_dict(self) -> dict:
        return {
            "title": self.title,
            "url": self.url,
            "employer": self.employer,
            "description": self.description,
        }


class VacancySearchService:
    """Сервис для поиска вакансий на HH.ru."""

    def __init__(self) -> None:
        self._settings = get_settings()

    async def _get_vacancy_description(self, page: Page, url: str) -> str:
        """
        Переход на страницу вакансии и извлечение полного описания.

        Аргументы:
            page: Страница браузера для использования.
            url: URL вакансии.

        Возвращает:
            Полный текст описания вакансии.
        """
        try:
            await page.goto(url, wait_until="domcontentloaded", timeout=15000)
            description_el = page.locator("[data-qa='vacancy-description']")
            await description_el.wait_for(state="visible", timeout=10000)

            if await description_el.count() > 0:
                return (await description_el.inner_text()).strip()

        except PlaywrightTimeoutError:
            logger.warning("Timed out while getting description for %s", url)
        except Exception as e:
            logger.warning("Failed to get description for %s: %s", url, e)

        return ""

    async def _check_bot_protection(self, page: Page) -> bool:
        """Проверка, показывает ли HH.ru страницу CAPTCHA/anti-bot."""
        title = (await page.title()).lower()

        captcha_selectors = [
            "iframe[src*='captcha']",
            "input[name='captcha']",
            "[data-qa*='captcha']",
            ".captcha",
            "text=Подтвердите, что вы не робот",
            "text=Я не робот",
            "text=not a robot",
        ]

        if "captcha" in title:
            return True

        for selector in captcha_selectors:
            try:
                locator = page.locator(selector)
                if await locator.count() > 0:
                    first_match = locator.first
                    if await first_match.is_visible(timeout=1000):
                        return True
            except Exception:
                continue

        return False

    async def search(
        self,
        query: Optional[str] = None,
        page_num: int = 0,
    ) -> list[dict]:
        """
        Поиск вакансий, соответствующих запросу.

        Аргументы:
            query: Текст запроса. По умолчанию используется значение из настроек.
            page_num: Номер страницы для пагинации (начиная с 0).

        Возвращает:
            Список словарей вакансий с заголовком, URL, работодателем и описанием.

        Исключения:
            RuntimeError: Если сработала защита от ботов.
            FileNotFoundError: Если файл сессии не найден.
        """
        query = query or self._settings.default_search_text
        encoded_query = quote_plus(query)

        logger.info("Searching vacancies: query='%s', page=%s", query, page_num)

        async with browser_manager.get_page(use_session=True) as page:
            url = (
                "https://hh.ru/search/vacancy?"
                f"text={encoded_query}&area={self._settings.area_code}"
                f"&items_on_page=20&page={page_num}"
            )

            await page.goto(url, wait_until="domcontentloaded", timeout=30000)

            if await self._check_bot_protection(page):
                await page.screenshot(path="captcha_debug.png", full_page=True)
                raise RuntimeError("Bot protection triggered (captcha detected)")

            await page.wait_for_selector(
                "[data-qa='vacancy-serp__vacancy']",
                timeout=10000,
            )

            vacancy_data: list[dict] = []
            cards = await page.locator("[data-qa='vacancy-serp__vacancy']").all()

            for i, card in enumerate(cards):
                try:
                    title_el = card.locator("[data-qa='serp-item__title']").first
                    await title_el.wait_for(state="visible", timeout=5000)

                    href = await title_el.get_attribute("href")
                    title = (await title_el.inner_text()).strip()

                    if not href:
                        logger.warning("Vacancy card %s has no href", i)
                        continue

                    employer_el = card.locator(
                        "[data-qa='vacancy-serp__vacancy-employer'], "
                        "[data-qa='vacancy-serp__vacancy-employer-text']"
                    ).first

                    employer = "Unknown"
                    if await employer_el.count() > 0:
                        employer = (await employer_el.inner_text()).strip() or "Unknown"

                    vacancy_data.append(
                        {
                            "title": title,
                            "url": urljoin("https://hh.ru", href),
                            "employer": employer,
                        }
                    )

                except Exception as e:
                    logger.warning("Failed to parse vacancy card %s: %s", i, e)
                    continue

            vacancies: list[dict] = []
            for data in vacancy_data:
                description = await self._get_vacancy_description(page, data["url"])
                vacancy = Vacancy(
                    title=data["title"],
                    url=data["url"],
                    employer=data["employer"],
                    description=description,
                )
                vacancies.append(vacancy.to_dict())

            logger.info("Found %s vacancies", len(vacancies))
            return vacancies
