# Troubleshooting

## ChromeDriver Unavailable

Start ChromeDriver and leave it running:

```powershell
chromedriver --port=9515
```

Verify it responds:

```powershell
Invoke-RestMethod http://localhost:9515/status
```

## Browser Profile Already In Use

Close Chrome windows using the configured profile directory. If you need separate login and API profiles, set:

```env
BROWSER_PROFILE_DIR=browser_profile_api
```

## CAPTCHA Detected

The API returns HTTP `503` with:

```json
{ "detail": "captcha detected" }
```

Open the browser profile manually, resolve the CAPTCHA, slow down the workflow, then retry. CAPTCHA screenshots are local artifacts and must not be committed.

## Timeouts

Increase `PAGE_TIMEOUT`, check that HH.ru loads in Chrome, and try `include_description=false` for faster searches. Full descriptions require opening each vacancy page and are more likely to hit slow pages or anti-bot checks.
