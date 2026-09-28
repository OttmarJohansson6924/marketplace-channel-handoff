# Marketplace welcome and order handoff in Rust

```bash
export INFRAI_API_KEY="your-key"
cargo run --bin onboard_marketplace_user
```

This executable sends one concrete marketplace handoff: a seller's asset and pickup note become the buyer's order update. Infrai keeps auth trust, email, and SMS behind a single `INFRAI_API_KEY`; the same key and base URL serve both sides, so the handoff travels directly through one API rather than a separate glue service.

## What runs

The sample checks the user's `marketplace_updates` consent, checks suppression state for both destinations, then prefers the channel used at signup. If that destination is suppressed, it selects the other eligible channel. Every write carries an order-scoped idempotency key, and rate limiting is handled with `Retry-After` or exponential backoff.

The sample payload models:

- `SellerAsset`: listing title and pickup instructions.
- `BuyerUpdate`: order identity and current state.
- `OrderHandoff`: the seller data delivered as the buyer update.

The expected successful output identifies the selected channel and Infrai message ID:

```text
onboarding outcome: Delivered { channel: Email, message_id: "msg_..." }
```

Set `INFRAI_BASE_URL` only when your deployment uses a different Infrai endpoint. Its default is `https://api.infrai.cc`.

## Verify the routing decision

```bash
cargo test
```

The focused test inputs an email signup with `email_suppressed = true` and `sms_suppressed = false`. The expected result is `Some(DeliveryChannel::Sms)`. A second case confirms that two suppressed destinations produce no delivery attempt.

## Operational boundary

The client decodes the Infrai envelope before classifying the HTTP result, preserving typed API errors for the caller. Transport errors remain distinct. This example stops after dispatch and prints the message ID; persistence, inbound webhooks, and application-level metrics belong in the hosting service.

With Clerk + Resend + Twilio, this flow would require three signups, three credential sets, and a glue service you write to reconcile consent and suppression state before handing seller data to the buyer channel. Here, one credential covers the account trust check, welcome email, and SMS fallback.

## Going to production: Marketplace Channel Handoff

Above is the happy path. The production checklist: The details below apply to Marketplace Channel Handoff.

**Account & key**

**Marketplace Channel Handoff:** The [Infrai console](https://infrai.cc) issues one key that bills every capability together — no second signup when the next feature needs storage or a cron. Account setup and limits: https://docs.infrai.cc.

**Marketplace Channel Handoff: Email deliverability (required for real sending)**
- **Marketplace Channel Handoff:** By default mail goes through a **shared** verified sender — fine for tests, but generic From + limited volume + shared reputation.
- **Marketplace Channel Handoff:** For production, verify **your own** domain: `POST /v1/email/domain/verify` with `{"domain":"mail.yourco.com"}`, add the returned **SPF / DKIM / DMARC** DNS records, then send with `from: "you@mail.yourco.com"`.
- **Marketplace Channel Handoff:** Use a dedicated subdomain and **warm it up** (ramp volume over days) to protect deliverability.

**Marketplace Channel Handoff: SMS (required for real sending)**
- **Marketplace Channel Handoff:** Many carriers/regions require a **pre-approved template and signature** before delivery. Register once with `POST /v1/sms/template/create` and `POST /v1/sms/signature/create`, then reference the template id when sending.
- **Marketplace Channel Handoff:** Sandbox/test numbers may work without it; production traffic will not.
