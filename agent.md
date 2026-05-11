Prepare this Rust project for a clean private GitHub push.

Context:
This is a Rust-based HH.ru automation backend used with n8n. It uses ChromeDriver/WebDriver, a persistent browser profile, manual login, /search, and /apply endpoints. The goal is to clean the repo so it can be safely pushed to a private GitHub repository and cloned later without exposing secrets or local machine artifacts.

Tasks:

1. Review the repository for sensitive or local-only files.
   - Do not commit real .env files.
   - Do not commit browser profiles, cookies, sessions, screenshots, logs, or credentials.
   - Do not commit Google/n8n tokens or HH.ru session data.

2. Create or update .gitignore.
   Ensure it excludes at least:
   .env
   browser_profile/
   browser_profile_api/
   captcha_debug.png
   apply_*.png
   *.log
   target/
   .DS_Store
   Thumbs.db

3. Create or update .env.example.
   Include safe placeholder values only:
   BROWSER_HEADLESS=false
   BROWSER_DRIVER_URL=http://localhost:9515
   BROWSER_PROFILE_DIR=browser_profile
   AREA_CODE=113
   ITEMS_PER_PAGE=10
   PAGE_TIMEOUT=30000
   HH_LOCALE=EN
   SERVER_HOST=127.0.0.1
   SERVER_PORT=3000

4. Update README.md.
   The README should clearly document:
   - project overview
   - requirements: Rust stable, Google Chrome, compatible ChromeDriver
   - environment setup
   - how to start ChromeDriver
   - how to run manual login using cargo run -- login
   - how to run the API using cargo run
   - /health endpoint
   - /search endpoint with both query and text aliases
   - /apply endpoint payload and response
   - n8n integration notes
   - troubleshooting section for ChromeDriver unavailable, browser profile already in use, CAPTCHA detected, and timeouts
   - warning that the tool uses a real browser profile and should be used slowly/manual-review-first

5. Add documentation files if useful:
   docs/n8n-workflow.md
   docs/troubleshooting.md

6. If there is an exported n8n workflow JSON, move it to:
   n8n/workflow-apply-review.json
   Ensure it does not contain credentials, tokens, personal sheet IDs, or secrets.

7. Run formatting and checks:
   cargo fmt
   cargo check
   cargo test if tests exist

8. Confirm Cargo.lock is committed because this is an application project.

9. Do not change core application behavior unless required for cleanup.
   Preserve:
   - /search behavior
   - /apply behavior
   - ChromeDriver integration
   - persistent browser profile config
   - n8n-compatible API contract

10. At the end, provide a summary of:
   - files changed
   - files ignored
   - any cleanup risks found
   - commands run
   - whether the repo is ready to push