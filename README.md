# rusthunter

Rust-based HH.ru automation backend for n8n workflows. It uses ChromeDriver/WebDriver with a persistent Chrome browser profile, so `/search` and `/apply` run through the same authenticated browser session you log in with manually.

Use this tool slowly and with manual review first. It uses a real browser profile, stores login state locally, and can trigger CAPTCHA or rate limits if requests are run too aggressively.

## Requirements

- Rust stable
- Google Chrome
- ChromeDriver compatible with your installed Chrome version
- n8n, if you want to run the workflow integration

## Environment Setup

Copy the example file and adjust local values:

```powershell
Copy-Item .env.example .env
```

Safe example values:

```env
BROWSER_HEADLESS=false
BROWSER_DRIVER_URL=http://localhost:9515
BROWSER_PROFILE_DIR=browser_profile
AREA_CODE=113
ITEMS_PER_PAGE=10
PAGE_TIMEOUT=30000
HH_LOCALE=EN
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
```

`BROWSER_PROFILE_DIR` is Chrome's persistent profile directory. It contains cookies and HH.ru login state, so it is ignored by git and must not be pushed.

`ITEMS_PER_PAGE` controls the HH.ru `items_on_page` query value. Keep it conservative for normal use.

## Start ChromeDriver

Start ChromeDriver and keep that terminal open:

```powershell
chromedriver --port=9515
```

Optional quieter logging:

```powershell
chromedriver --port=9515 --log-level=OFF --silent
```

Verify it is reachable:

```powershell
Invoke-RestMethod http://localhost:9515/status
```

## Manual Login

In a second terminal, open the login browser:

```powershell
cargo run -- login
```

Log in to HH.ru in the Chrome window. After login completes, press `Ctrl+C` in the terminal. The browser profile is reused by API requests.

## Run API

With ChromeDriver still running:

```powershell
cargo run
```

Or use the helper script:

```powershell
.\scripts\start.ps1
```

## Endpoints

Health check:

```text
GET http://127.0.0.1:3000/health
```

Search with the `query` alias:

```text
GET http://127.0.0.1:3000/search?query=AI%20Automation%20Engineer&page=1
```

Search with the n8n-friendly `text` alias:

```text
GET http://127.0.0.1:3000/search?text=AI%20Automation%20Engineer&page=1
```

Include or skip full vacancy descriptions:

```text
GET http://127.0.0.1:3000/search?text=AI%20Automation%20Engineer&page=1&include_description=true
GET http://127.0.0.1:3000/search?text=AI%20Automation%20Engineer&page=1&include_description=false
```

`include_description` defaults to `true`. When enabled, the service opens each vacancy page with a conservative delay and fills the `description` field when extraction succeeds.

Successful search response:

```json
[
  {
    "title": "AI Automation Engineer",
    "url": "https://hh.ru/vacancy/...",
    "employer": "Company Name",
    "description": "Full vacancy description..."
  }
]
```

Apply to vacancy:

```text
POST http://127.0.0.1:3000/apply
Content-Type: application/json
```

```json
{
  "vacancy_url": "https://hh.ru/vacancy/...",
  "cover_letter": "Generated cover letter text",
  "resume_id": "optional"
}
```

`url` is also accepted instead of `vacancy_url` for n8n rows.

The backend opens the vacancy page in the persistent visible Chrome profile, clicks the reply/apply button, fills the cover letter if a textarea is available, then stops before final submission and leaves the browser open for manual review. The HTTP response is returned only after you manually close the browser window.

Successful review completion response keeps fields from the input item and changes `status` to `applied`:

```json
{
  "row_number": 8,
  "title": "Example vacancy",
  "url": "https://hh.ru/vacancy/...",
  "employer": "Company Name",
  "description": "Full vacancy description...",
  "cover_letter": "Generated cover letter text",
  "status": "applied"
}
```

Failed apply response:

```json
{
  "status": "failed",
  "detail": "Reason",
  "applied": false
}
```

Common error response:

```json
{
  "detail": "Description of error"
}
```

Important status codes:

```text
400 invalid input
409 browser profile already in use
502 ChromeDriver/browser failure
503 CAPTCHA detected
504 page timeout
```

## n8n Integration

Use an HTTP Request node for search:

```text
Method: GET
URL: http://127.0.0.1:3000/search
Query:
  text = {{$json.searchText}}
  page = 1
  include_description = true
```

Use an HTTP Request node for apply:

```text
Method: POST
URL: http://127.0.0.1:3000/apply
Body JSON:
{
  "vacancy_url": "{{ $json.url }}",
  "cover_letter": "{{ $json.cover_letter }}",
  "resume_id": "{{ $json.resume_id }}"
}
```

A sanitized workflow export is available at `n8n/workflow-apply-review.json`. Configure your own Google Sheets document and n8n credentials after importing it.

## Troubleshooting

ChromeDriver unavailable: start ChromeDriver with `chromedriver --port=9515` and verify `http://localhost:9515/status`.

Browser profile already in use: close Chrome windows using the configured profile, or set `BROWSER_PROFILE_DIR=browser_profile_api`.

CAPTCHA detected: the service returns HTTP `503` and saves a screenshot under the local browser profile screenshots directory. Resolve the CAPTCHA manually in the browser, slow down the workflow, then retry.

Timeouts: increase `PAGE_TIMEOUT`, verify HH.ru is reachable in the browser, and try `include_description=false` for faster searches.

## Repository Hygiene

Do not commit real `.env` files, browser profiles, cookies, screenshots, logs, credentials, Google tokens, n8n tokens, HH.ru sessions, or local build output. `Cargo.lock` should be committed because this is an application project.
