# n8n Workflow

This project includes a sanitized n8n export at `n8n/workflow-apply-review.json`.

## Import

1. Import `n8n/workflow-apply-review.json` into n8n.
2. Configure your own Google Sheets credential on the Google Sheets node.
3. Replace `YOUR_SPREADSHEET_ID` with your own sheet URL or select the document in the n8n UI.
4. Confirm your sheet has rows with `url`, `cover_letter`, `resume_id`, and `status` fields.
5. Keep rows at `status=ready` when they should be processed.

## Backend Calls

Search uses:

```text
GET http://127.0.0.1:3000/search
```

Apply uses:

```text
POST http://127.0.0.1:3000/apply
```

The apply workflow is intentionally manual-review-first. The backend opens HH.ru in Chrome, prepares the response, and waits for you to close the browser window before returning.

## Safety

Do not export credentials, tokens, personal sheet IDs, or HH.ru session data into this repository. Keep local workflow exports private unless they have been scrubbed.
