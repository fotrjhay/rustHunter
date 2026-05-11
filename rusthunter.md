# RUST_JOB_SEARCH_BACKEND_SPEC.md

## 1. Concept Overview

This project is a **standalone Rust-based job search backend service** that simulates human browsing behavior to retrieve job listings from HH.ru and exposes them via a clean HTTP API.

It is intentionally designed as a **browser-driven system**, not a simple HTTP scraper, in order to:

* Respect session-based access patterns
* Reduce bot detection
* Mimic real user interaction

This is a **fresh system design**, independent of any existing implementation.

Once stable, it will be integrated into an existing **n8n automation workflow** as a drop-in replacement backend.

---

## 2. System Role

```text
n8n (or any client)
        ↓
Rust Job Search API (this service)
        ↓
Browser Automation Layer (Chrome/WebDriver)
        ↓
HH.ru (external system)
```

The backend is responsible for:

* Managing browser sessions
* Performing search queries
* Extracting structured data
* Handling anti-bot scenarios safely

---

## 3. Design Philosophy

### Human-like over fast

Speed is not the goal. Stability is.

### Session over stateless

Requests must reuse a real browsing session.

### Minimal surface scraping

Only collect what is necessary from the search page.

### Fail clearly, not silently

Errors must be observable and debuggable.

---

## 4. Technology Stack

| Layer              | Tool                     |
| ------------------ | ------------------------ |
| Web API            | `axum`                   |
| Async runtime      | `tokio`                  |
| Serialization      | `serde`                  |
| Logging            | `tracing`                |
| Config             | `config` / `envy`        |
| Browser automation | `thirtyfour` (WebDriver) |
| Driver             | ChromeDriver             |

---

## 5. Project Structure

```text
src/
├── main.rs
├── config.rs
├── errors.rs
├── models.rs
├── routes/
│   ├── mod.rs
│   └── search.rs
├── services/
│   ├── mod.rs
│   ├── browser.rs
│   └── job_search.rs
└── utils/
    ├── delay.rs
    ├── captcha.rs
    └── session.rs
```

---

## 6. API Design (Initial)

### Health Check

```http
GET /health
```

Response:

```json
{ "status": "ok" }
```

---

### Search Jobs

```http
GET /search?query=<text>&page=<number>
```

Example:

```http
/search?query=AI%20Automation%20Engineer&page=1
```

Response:

```json
[
  {
    "title": "AI Automation Engineer",
    "url": "https://hh.ru/vacancy/...",
    "employer": "Company Name",
    "description": ""
  }
]
```

### Apply to Vacancy

```http
POST /apply
```

Request:

```json
{
  "vacancy_url": "https://hh.ru/vacancy/...",
  "cover_letter": "Generated cover letter text",
  "resume_id": "optional"
}
```

Success:

```json
{
  "status": "success",
  "message": "Application submitted",
  "vacancy_url": "https://hh.ru/vacancy/...",
  "applied": true
}
```

Failure:

```json
{
  "status": "failed",
  "detail": "Reason",
  "applied": false
}
```

---

## 7. Core Workflow

### Step 1: Receive request

* Parse query and page number
* Validate inputs

---

### Step 2: Initialize browser session

* Load persistent session profile (cookies, auth)
* If session missing → return error

---

### Step 3: Apply human-like delay

```text
Wait 5 seconds
```

---

### Step 4: Navigate to search page

Construct URL:

```text
https://hh.ru/search/vacancy?text=<encoded>&area=<code>&items_on_page=20&page=<page>
```

---

### Step 5: Detect bot protection

Check ONLY:

* CAPTCHA iframe
* CAPTCHA input field
* Visible "I am not a robot" indicators

If detected:

* Capture screenshot
* Return HTTP 503

---

### Step 6: Extract data

From search results page only:

* title
* url
* employer

DO NOT:

* open each vacancy page (default)

---

### Step 7: Return structured JSON

---

## 8. Anti-Bot Strategy

### Mandatory Rules

* Fixed 5-second delay before navigation
* No rapid pagination
* No bulk detail-page scraping
* Use authenticated session

---

### CAPTCHA Handling

If CAPTCHA detected:

1. Save screenshot:

   ```text
   browser_profile/screenshots/captcha_debug.png
   ```
2. Return:

   ```http
   503 Service Unavailable
   ```
3. Do NOT retry automatically

---

## 9. Session Management

### Requirements

* Persistent session storage
* Reusable across runs
* Manual login support

### Expected flow

```text
Run login command → browser opens → user logs in → session saved
```

---

## 10. Browser Layer

### Requirements

* Must support:

  * navigation
  * DOM queries
  * cookies/session reuse

### Mode

* Default: `headless = false` (for stability)
* Configurable via environment

---

## 11. Configuration (.env)

```env
BROWSER_HEADLESS=false
AREA_CODE=113
ITEMS_PER_PAGE=20
PAGE_TIMEOUT=30000
HH_LOCALE=EN
```

---

## 12. Error Handling

| Scenario         | Response  |
| ---------------- | --------- |
| CAPTCHA detected | 503       |
| Session missing  | 500       |
| Timeout          | 504       |
| Partial parsing  | skip item |

All responses must include:

```json
{
  "detail": "Description of error"
}
```

---

## 13. Logging

Log:

* incoming request
* query + page
* navigation steps
* number of results
* errors

---

## 14. Development Phases

### Phase 1: API skeleton

* `/health`
* `/search` (mock data)

### Phase 2: Browser integration

* ChromeDriver setup
* open search page

### Phase 3: Parsing

* extract results

### Phase 4: Anti-bot stability

* delays
* CAPTCHA detection
* screenshots

### Phase 5: Refinement

* optional detail scraping
* selector robustness

---

## 15. Integration with n8n (Future Step)

Once stable, align API to match existing workflow:

```text
GET /search?text=<query>&page=<page>
```

Mapping:

```text
query → text
```

Response format must match exactly.

---

## 16. Success Criteria

* Works reliably without triggering CAPTCHA frequently
* Mimics human browsing behavior
* Returns structured job data
* Integrates seamlessly with n8n
* Failures are debuggable via logs + screenshots

---

## 17. Final Principle

This is not just a scraper.

It is a **controlled browser automation system designed to behave like a human user**.

Every design decision must follow:

```text
stealth > speed
stability > completeness
```
