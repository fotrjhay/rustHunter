Implement the missing “Apply to Vacancy” backend feature.

Context:
The existing n8n workflow already has a POST node called “Apply to Vacancy” after “Prepare Apply Data”. The backend must expose a POST /apply endpoint that receives prepared application data from n8n and uses the existing browser/session automation layer to apply/reply to a vacancy on HH.ru.

Requirements:
1. Add POST /apply endpoint.
2. Expected JSON body:
{
  "vacancy_url": "https://hh.ru/vacancy/...",
  "cover_letter": "Generated cover letter text",
  "resume_id": "optional"
}

3. The endpoint must:
- open vacancy_url using the saved authenticated browser session
- wait 5 seconds before interaction
- detect CAPTCHA using the existing strict selector-based detection logic
- click the HH.ru apply/respond button
- wait for the reply/application modal or form
- fill the cover letter textarea if available
- select resume if resume_id is provided and the UI supports it
- submit the application
- confirm success from the page/modal
- return structured JSON

4. Response format:
Success:
{
  "status": "success",
  "message": "Application submitted",
  "vacancy_url": "...",
  "applied": true
}

Failure:
{
  "status": "failed",
  "detail": "Reason",
  "applied": false
}

5. Do not use raw direct HH.ru POST requests unless the existing authenticated browser flow requires it. Prefer browser-driven interaction.

6. Add screenshots for debugging:
- browser_profile/screenshots/apply_before_click.png
- browser_profile/screenshots/apply_modal.png
- browser_profile/screenshots/apply_success.png
- browser_profile/screenshots/apply_error.png

7. Do not change the existing /search contract used by n8n.

8. Add clear logs for each step.

9. Keep rate limiting conservative. No bulk rapid applications.

10. If CAPTCHA is detected, return HTTP 503 and do not retry automatically.

Implement this in the existing backend architecture, reusing browser_manager and the existing session handling.
