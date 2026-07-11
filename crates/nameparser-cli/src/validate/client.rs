// SPDX-License-Identifier: Apache-2.0

//! LLM judge backends — mirrors the Java CLI's `llm.{Judge,AnthropicClient,OpenAiClient}`
//! (`/Users/markus/code/gbif/name-parser/name-parser-cli/src/main/java/org/gbif/nameparser/cli/llm/`).
//! See the recon doc's §6 for the full verified per-provider request/response/auth/retry
//! contract this reproduces.
//!
//! Both clients are built on `ureq` (a synchronous HTTP client with no async-runtime
//! dependency, matching this whole CLI's synchronous design and Java's own single-threaded,
//! sequential-batch judging) + hand-rolled retry/backoff (matching Java, which also hand-rolls
//! it on top of the JDK's `HttpClient` rather than pulling in a retry library) — no
//! Anthropic/OpenAI SDK dependency, exactly like the Java side.
//!
//! The HTTP-transport code itself ([`Judge::judge`]'s request/retry loop) has no automated test
//! in this task (no live API calls, per this port's testing policy) — but every part of it that
//! *can* be tested without a network call is extracted into its own free function and unit
//! tested below: response extraction ([`extract_anthropic_verdicts`], [`extract_openai_content`],
//! [`openai_finish_reason`], [`parse_openai_reply`]), the retry decision itself
//! ([`retry_decision`], [`parse_retry_after_secs`]), the request-body shape ([`verdict_schema`],
//! both clients' `request_body`), and provider/model resolution ([`build_judge`]).

use std::time::Duration;

use super::{ValidationPrompt, Verdict};

/// A backend that judges one batch — Java `llm.Judge`. Implemented by [`AnthropicClient`]
/// (cloud) and [`OpenAiClient`] (local, OpenAI-compatible servers such as Ollama, LM Studio,
/// llama.cpp), so the (Task 5) judge loop is agnostic to where the model runs.
pub trait Judge {
    /// Judge one batch. `user_message` is [`ValidationPrompt::user_message`]'s payload;
    /// `batch_size` sizes the token budget only — chunking `chosen` into batches, and further
    /// splitting a batch into cached/uncached names, is entirely the (Task 5) caller's
    /// responsibility: one `judge` call is always exactly one HTTP request, matching Java's
    /// `Judge.judge(String, int)` contract precisely.
    fn judge(&self, user_message: &str, batch_size: usize) -> Result<Vec<Verdict>, JudgeError>;

    /// The model id this judge was constructed with. Not present on Java's `Judge` interface
    /// (Java code that needs the model id just closes over the concrete client instead) — added
    /// here so a caller holding a `Box<dyn Judge>` (e.g. [`build_judge`]'s return value, or a
    /// future startup log line / this module's own tests) can read it back without downcasting.
    fn model_id(&self) -> &str;
}

/// Shared `max_tokens` formula both clients' request bodies use (Java: `AnthropicClient`'s and
/// `OpenAiClient`'s `requestBody` both compute `Math.min(32000, 2000 + batchSize *
/// maxTokensPerName)` with the identical hard-coded `maxTokensPerName = 400`) — headroom for
/// roughly one verdict per name plus fixed overhead, capped so a huge batch can't request an
/// unbounded budget.
pub fn judge_max_tokens(batch_size: usize) -> u32 {
    (2000 + batch_size as u32 * 400).min(32000)
}

/// Error from a [`Judge::judge`] call — collapses Java's checked `IOException`/
/// `InterruptedException` (thrown when retries are exhausted, a non-retryable HTTP status is
/// hit, the response body is malformed, or the request transport itself fails) into one error
/// type, since this port has no checked exceptions to mirror them with individually.
#[derive(Debug)]
pub struct JudgeError(pub String);

impl std::fmt::Display for JudgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for JudgeError {}

// ---------------------------------------------------------------------------------------
// Retry / backoff — shared by both clients, unit-testable without any network I/O
// ---------------------------------------------------------------------------------------

/// Java `AnthropicClient`/`OpenAiClient`'s shared `MAX_ATTEMPTS`.
pub const MAX_JUDGE_ATTEMPTS: u32 = 4;

/// The retry decision for one failed HTTP attempt — mirrors both clients' shared retry contract
/// (recon §5/§6) as a single pure, HTTP-library-agnostic function: HTTP 429 or >=500 retries
/// (up to [`MAX_JUDGE_ATTEMPTS`]), honoring a positive `retry_after_secs` hint if the caller has
/// one (Anthropic only reads a `retry-after` response header; OpenAI-compatible always passes
/// `None`, matching Java's asymmetry — `OpenAiClient` has no `retry-after` support at all);
/// every other status returns `None` (give up immediately, no retry).
///
/// `attempt` is the 1-based number of the attempt that just failed (matching Java's `for (int
/// attempt = 1; attempt <= MAX_ATTEMPTS; attempt++)` loop variable exactly, including its
/// backoff formula's use of the CURRENT attempt number, not a zero-based or "next attempt"
/// count). Returns the backoff [`Duration`] to sleep before retrying, or `None` if the caller
/// should give up and surface an error instead.
pub fn retry_decision(
    status: u16,
    attempt: u32,
    retry_after_secs: Option<u64>,
) -> Option<Duration> {
    if !(status == 429 || status >= 500) {
        return None;
    }
    if attempt >= MAX_JUDGE_ATTEMPTS {
        return None;
    }
    if let Some(secs) = retry_after_secs {
        if secs > 0 {
            return Some(Duration::from_secs(secs));
        }
    }
    // Java `Math.pow(2, attempt) * 500L`.
    Some(Duration::from_millis(2u64.pow(attempt) * 500))
}

/// Parses an HTTP `retry-after` header value into a positive whole-second count, or `None` if
/// absent/unparsable/non-positive. Mirrors Java `AnthropicClient.retryAfterMillis`'s
/// `Long.parseLong(s.trim())` + `.filter(ms -> ms != null && ms > 0)` (there, converted to
/// milliseconds immediately; here, kept in seconds since [`retry_decision`] takes seconds
/// directly and converts itself). Java's `Long.parseLong` can parse a negative number (then
/// filters it out via `ms > 0`); Rust's `u64::from_str` instead simply fails to parse a leading
/// `-` at all — a different mechanism, but the same observable outcome: any non-positive or
/// unparsable value falls back to the exponential-backoff formula either way.
pub(crate) fn parse_retry_after_secs(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok().filter(|&s| s > 0)
}

// ---------------------------------------------------------------------------------------
// AnthropicClient
// ---------------------------------------------------------------------------------------

const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Minimal client for the Anthropic Messages API — Java `llm.AnthropicClient`. Sends one judging
/// batch per request and uses Anthropic's structured-output feature (`output_config.format`
/// with a JSON schema, see [`verdict_schema`]) so the reply is guaranteed to already be a JSON
/// object matching [`Verdict`]'s shape — no free-text parsing tolerance is strictly needed on
/// this path, though the reply is still funneled through [`super::parse_verdicts`] for a single
/// shared extraction code path with [`OpenAiClient`].
pub struct AnthropicClient {
    agent: ureq::Agent,
    endpoint: String,
    model: String,
    api_key: Option<String>,
    bearer_token: Option<String>,
}

impl AnthropicClient {
    /// Java `ValidateCli`'s documented cloud default (`VALIDATE.md`'s option table).
    pub const DEFAULT_MODEL: &'static str = "claude-opus-4-8";

    /// Java `AnthropicClient.fromEnv(String, String)`: `api_url` (from `--api-url`) takes
    /// priority over `ANTHROPIC_BASE_URL`, which takes priority over the public API default.
    /// Auth prefers `ANTHROPIC_API_KEY` (sent as `x-api-key`); if unset, falls back to
    /// `ANTHROPIC_AUTH_TOKEN` (a bearer token, e.g. from `ant auth print-credentials
    /// --access-token`, sent as `authorization: Bearer ...` plus an `anthropic-beta` header);
    /// if NEITHER is set, returns a [`JudgeError`] explaining both options (and `--dry-run` as
    /// the escape hatch that needs no credential at all) rather than constructing a client that
    /// could never actually authenticate.
    pub fn from_env(model: &str, api_url: Option<&str>) -> Result<Self, JudgeError> {
        let base = api_url
            .map(str::to_string)
            .unwrap_or_else(|| env_or("ANTHROPIC_BASE_URL", "https://api.anthropic.com"));
        let key = env_non_blank("ANTHROPIC_API_KEY");
        let token = env_non_blank("ANTHROPIC_AUTH_TOKEN");
        if key.is_none() && token.is_none() {
            return Err(JudgeError(
                "No Anthropic credential found. Set ANTHROPIC_API_KEY, or export a bearer token:\n"
                    .to_string()
                    + "  export ANTHROPIC_AUTH_TOKEN=$(ant auth print-credentials --access-token)\n"
                    + "Alternatively run with --dry-run to build batches without calling the API.",
            ));
        }
        Ok(AnthropicClient {
            agent: ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(30))
                .timeout(Duration::from_secs(5 * 60))
                .build(),
            endpoint: format!("{}/v1/messages", trim_trailing_slash(&base)),
            model: model.to_string(),
            api_key: key,
            bearer_token: token,
        })
    }

    /// Java `AnthropicClient.requestBody(String, int)`.
    fn request_body(&self, user_message: &str, batch_size: usize) -> serde_json::Value {
        serde_json::json!({
            "model": self.model,
            "max_tokens": judge_max_tokens(batch_size),
            "thinking": {"type": "adaptive"},
            "system": ValidationPrompt::SYSTEM,
            "messages": [{"role": "user", "content": user_message}],
            "output_config": {
                "format": {
                    "type": "json_schema",
                    "schema": verdict_schema(),
                },
            },
        })
    }
}

impl Judge for AnthropicClient {
    fn judge(&self, user_message: &str, batch_size: usize) -> Result<Vec<Verdict>, JudgeError> {
        let body = serde_json::to_string(&self.request_body(user_message, batch_size))
            .expect("a serde_json::Value built from json! always serializes");

        let mut attempt = 1u32;
        loop {
            let mut req = self
                .agent
                .post(&self.endpoint)
                .set("content-type", "application/json")
                .set("anthropic-version", ANTHROPIC_VERSION);
            req = match &self.api_key {
                Some(key) => req.set("x-api-key", key),
                None => req
                    .set(
                        "authorization",
                        &format!(
                            "Bearer {}",
                            self.bearer_token.as_deref().unwrap_or_default()
                        ),
                    )
                    .set("anthropic-beta", "oauth-2025-04-20"),
            };

            match req.send_string(&body) {
                Ok(resp) if resp.status() == 200 => {
                    let text = resp
                        .into_string()
                        .map_err(|e| JudgeError(format!("reading Anthropic response body: {e}")))?;
                    return extract_anthropic_verdicts(&text)
                        .map_err(|e| JudgeError(e.to_string()));
                }
                // ureq only ever returns `Ok` for status < 400; a 200-399 non-200 (e.g. a
                // redirect ureq didn't resolve to a final response) is not itself an
                // Anthropic-API-shaped success — treat it like Java's `sc == 200` check does:
                // anything other than exactly 200 falls straight to the non-retryable-error path.
                Ok(resp) => {
                    let code = resp.status();
                    let body_text = resp.into_string().unwrap_or_default();
                    return Err(JudgeError(format!(
                        "Anthropic API error {code}: {}",
                        super::brief(&body_text)
                    )));
                }
                Err(ureq::Error::Status(code, resp)) => {
                    let retry_after = resp.header("retry-after").and_then(parse_retry_after_secs);
                    let body_text = resp.into_string().unwrap_or_default();
                    match retry_decision(code, attempt, retry_after) {
                        Some(wait) => {
                            std::thread::sleep(wait);
                            attempt += 1;
                        }
                        None => {
                            return Err(JudgeError(format!(
                                "Anthropic API error {code}: {}",
                                super::brief(&body_text)
                            )));
                        }
                    }
                }
                Err(ureq::Error::Transport(t)) => {
                    return Err(JudgeError(format!("Anthropic API transport error: {t}")));
                }
            }
        }
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

/// Java `AnthropicClient.verdictSchema()`: the JSON Schema constraining an Anthropic
/// structured-output reply to exactly `{"verdicts":[Verdict, ...]}` — transcribed field-for-field
/// via `serde_json::json!` rather than Java's hand-built `JsonObject` tree (recon §6/§10: no
/// schema-generation crate is needed or used on either side). Root object requires `verdicts`,
/// `additionalProperties: false`; each verdict object requires ALL of `index`/`verdict`/
/// `confidence`/`fields`/`note` (even `note`/`fields`, which can be blank/empty) and forbids
/// additional properties; `verdict`/`confidence` are constrained string `enum`s; each `fields[]`
/// issue requires all of `name`/`parsed`/`expected`/`reason` (all typed `string`) and also
/// forbids additional properties.
pub fn verdict_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "verdicts": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "index": {"type": "integer"},
                        "verdict": {"type": "string", "enum": ["ok", "suspect", "wrong"]},
                        "confidence": {"type": "string", "enum": ["low", "med", "high"]},
                        "fields": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": {"type": "string"},
                                    "parsed": {"type": "string"},
                                    "expected": {"type": "string"},
                                    "reason": {"type": "string"},
                                },
                                "required": ["name", "parsed", "expected", "reason"],
                                "additionalProperties": false,
                            },
                        },
                        "note": {"type": "string"},
                    },
                    "required": ["index", "verdict", "confidence", "fields", "note"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["verdicts"],
        "additionalProperties": false,
    })
}

/// Java `AnthropicClient.parseVerdicts(String)`: extract the structured-output JSON from a
/// Messages API response body — concatenate the `text` of every `content[]` block with `type ==
/// "text"` (skipping e.g. `thinking` blocks), then feed the concatenation through
/// [`super::parse_verdicts`]. Kept free of any HTTP-transport code so it's directly unit-testable
/// against a canned response body (recon: `AnthropicClientTest.parsesStructuredVerdicts`), unlike
/// [`Judge::judge`] itself.
pub fn extract_anthropic_verdicts(response_body: &str) -> std::io::Result<Vec<Verdict>> {
    let resp: serde_json::Value = serde_json::from_str(response_body).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid Anthropic response JSON: {e}"),
        )
    })?;
    let mut text = String::new();
    if let Some(blocks) = resp.get("content").and_then(|c| c.as_array()) {
        for block in blocks {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                    text.push_str(t);
                }
            }
        }
    }
    if text.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "No text block in Anthropic response: {}",
                super::brief(response_body)
            ),
        ));
    }
    super::parse_verdicts(&text)
}

// ---------------------------------------------------------------------------------------
// OpenAiClient — also serves --provider=local/ollama
// ---------------------------------------------------------------------------------------

/// [`Judge`] backed by any OpenAI-compatible `/v1/chat/completions` endpoint — Java
/// `llm.OpenAiClient`. Chiefly local runtimes (Ollama, LM Studio, llama.cpp), so validation can
/// run for free on a local machine instead of the metered cloud API; there is no separate
/// "local" client type on either the Java or Rust side, just this one defaulted to a local URL.
///
/// Requests `response_format: json_object` (a weak "must be valid JSON" constraint, not a
/// schema) and repeats the exact reply shape in the prompt
/// ([`ValidationPrompt::OUTPUT_INSTRUCTION`]), then parses the reply tolerantly
/// ([`super::parse_verdicts`]) since local models are looser than Claude about wrapping their
/// output. No API key is required for a typical local server.
pub struct OpenAiClient {
    agent: ureq::Agent,
    endpoint: String,
    model: String,
    api_key: Option<String>,
}

impl OpenAiClient {
    /// Java `ValidateCli`'s documented local default (`VALIDATE.md`'s option table;
    /// `qwen2.5:32b-instruct` there is only a "bigger option" example, not this coded default).
    pub const DEFAULT_MODEL: &'static str = "qwen2.5:14b-instruct";

    /// Java `OpenAiClient.fromEnv(String, String)`: `api_url` (from `--api-url`) takes priority
    /// over `OPENAI_BASE_URL`, which takes priority over Ollama's default
    /// (`http://localhost:11434`). `OPENAI_API_KEY` is optional and, unlike Anthropic, never
    /// fails construction if absent — local servers generally ignore/don't need it.
    pub fn from_env(model: &str, api_url: Option<&str>) -> Self {
        let base = api_url
            .map(str::to_string)
            .unwrap_or_else(|| env_or("OPENAI_BASE_URL", "http://localhost:11434"));
        OpenAiClient {
            agent: ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(30))
                .timeout(Duration::from_secs(10 * 60)) // local generation can be slow
                .build(),
            endpoint: format!("{}/v1/chat/completions", trim_trailing_slash(&base)),
            model: model.to_string(),
            api_key: env_non_blank("OPENAI_API_KEY"),
        }
    }

    /// Java `OpenAiClient.requestBody(String, int)`.
    fn request_body(&self, user_message: &str, batch_size: usize) -> serde_json::Value {
        serde_json::json!({
            "model": self.model,
            "temperature": 0,
            "max_tokens": judge_max_tokens(batch_size),
            "stream": false,
            "response_format": {"type": "json_object"},
            "messages": [
                {
                    "role": "system",
                    "content": format!(
                        "{}\n\n{}",
                        ValidationPrompt::SYSTEM,
                        ValidationPrompt::OUTPUT_INSTRUCTION
                    ),
                },
                {"role": "user", "content": user_message},
            ],
        })
    }
}

impl Judge for OpenAiClient {
    fn judge(&self, user_message: &str, batch_size: usize) -> Result<Vec<Verdict>, JudgeError> {
        let body = serde_json::to_string(&self.request_body(user_message, batch_size))
            .expect("a serde_json::Value built from json! always serializes");

        let mut attempt = 1u32;
        loop {
            let mut req = self
                .agent
                .post(&self.endpoint)
                .set("content-type", "application/json");
            if let Some(key) = &self.api_key {
                req = req.set("authorization", &format!("Bearer {key}"));
            }

            match req.send_string(&body) {
                Ok(resp) if resp.status() == 200 => {
                    let text = resp.into_string().map_err(|e| {
                        JudgeError(format!("reading OpenAI-compatible response body: {e}"))
                    })?;
                    return Ok(parse_openai_reply(&text, batch_size, &self.model));
                }
                Ok(resp) => {
                    let code = resp.status();
                    let body_text = resp.into_string().unwrap_or_default();
                    return Err(JudgeError(format!(
                        "OpenAI-compatible API error {code}: {}\nEndpoint: {} (is the local \
                         server running and the model pulled?)",
                        super::brief(&body_text),
                        self.endpoint
                    )));
                }
                Err(ureq::Error::Status(code, resp)) => {
                    let body_text = resp.into_string().unwrap_or_default();
                    // OpenAI-compatible path has no `retry-after` support in Java either —
                    // always the exponential-backoff formula.
                    match retry_decision(code, attempt, None) {
                        Some(wait) => {
                            std::thread::sleep(wait);
                            attempt += 1;
                        }
                        None => {
                            return Err(JudgeError(format!(
                                "OpenAI-compatible API error {code}: {}\nEndpoint: {} (is the \
                                 local server running and the model pulled?)",
                                super::brief(&body_text),
                                self.endpoint
                            )));
                        }
                    }
                }
                Err(ureq::Error::Transport(t)) => {
                    return Err(JudgeError(format!(
                        "OpenAI-compatible API transport error: {t}\nEndpoint: {} (is the local \
                         server running and the model pulled?)",
                        self.endpoint
                    )));
                }
            }
        }
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

/// Java `OpenAiClient.extractContent(String)`: pull `choices[0].message.content` out of an
/// OpenAI-compatible chat-completions reply. Errors (Java's `IllegalStateException`, ported as
/// an [`std::io::Error`]) if `choices` is missing/empty or `message`/`content` is absent.
pub fn extract_openai_content(response_body: &str) -> std::io::Result<String> {
    let resp: serde_json::Value = serde_json::from_str(response_body).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid OpenAI-compatible response JSON: {e}"),
        )
    })?;
    let no_choices = || {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("No choices in response: {}", super::brief(response_body)),
        )
    };
    let first = resp
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .ok_or_else(no_choices)?;
    first
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "No message content in response: {}",
                    super::brief(response_body)
                ),
            )
        })
}

/// Java `OpenAiClient.finishReason(String)`: `choices[0].finish_reason`, or `None` if
/// absent/unparseable (Java swallows any `RuntimeException` here and returns `null` — this port
/// uses `Option` chaining via `?`/`and_then`, which has the same "any missing piece => None"
/// effect without needing an explicit catch).
pub fn openai_finish_reason(response_body: &str) -> Option<String> {
    let resp: serde_json::Value = serde_json::from_str(response_body).ok()?;
    resp.get("choices")?
        .as_array()?
        .first()?
        .get("finish_reason")?
        .as_str()
        .map(str::to_string)
}

/// Java `OpenAiClient.parseReply(String, int)`: turn a 200 chat-completions body into verdicts,
/// resiliently. Local models are flaky — a batch may come back truncated at `max_tokens`, empty,
/// or otherwise unparseable — and a single bad batch must not abort the whole judging run. On
/// any such failure this `eprintln!`s a warning and returns an empty list: those names stay
/// unjudged and, because callers only cache non-null verdicts, uncached — so a later run retries
/// them. HTTP-level failures are NOT handled here (they never reach this function — see
/// [`Judge::judge`]'s own retry/error path, which surfaces those as a [`JudgeError`] exactly
/// like [`AnthropicClient`]'s does).
pub(crate) fn parse_openai_reply(
    response_body: &str,
    batch_size: usize,
    model: &str,
) -> Vec<Verdict> {
    if openai_finish_reason(response_body).as_deref() == Some("length") {
        eprintln!(
            "Model '{model}' hit max_tokens and truncated its reply for this batch of \
             {batch_size}; verdicts up to the cut-off are salvaged, the rest are left unjudged. \
             Use a smaller --batch or a less verbose model to judge them all."
        );
    }

    let content = match extract_openai_content(response_body) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Model '{model}' produced an unusable reply for this batch of {batch_size} \
                 ({e}); leaving these names unjudged (they stay uncached, so a later run \
                 retries them)."
            );
            return Vec::new();
        }
    };

    match super::parse_verdicts(&content) {
        Ok(verdicts) => {
            if verdicts.is_empty() {
                eprintln!(
                    "Model '{model}' returned no verdicts for this batch of {batch_size}; \
                     leaving these names unjudged (they stay uncached, so a later run retries \
                     them)."
                );
            }
            verdicts
        }
        Err(e) => {
            eprintln!(
                "Model '{model}' produced an unusable reply for this batch of {batch_size} \
                 ({e}); leaving these names unjudged (they stay uncached, so a later run \
                 retries them)."
            );
            Vec::new()
        }
    }
}

// ---------------------------------------------------------------------------------------
// build_judge — provider normalization + default-model resolution
// ---------------------------------------------------------------------------------------

/// Java `ValidateCli`'s provider resolution (`ValidateCli.java:79-81` normalizes `local`/
/// `ollama` to `"openai"`) plus each client's own `fromEnv` default-model fallback, combined
/// into one entry point: normalizes `provider` (case-insensitively), resolves `model` to the
/// provider's documented default when `--model` is unset ([`AnthropicClient::DEFAULT_MODEL`] /
/// [`OpenAiClient::DEFAULT_MODEL`]), and constructs the matching [`Judge`] from the environment.
/// An unrecognized provider string is a [`JudgeError`], not a panic.
pub fn build_judge(
    provider: &str,
    model: Option<&str>,
    api_url: Option<&str>,
) -> Result<Box<dyn Judge>, JudgeError> {
    match provider.to_ascii_lowercase().as_str() {
        "anthropic" => {
            let model = model.unwrap_or(AnthropicClient::DEFAULT_MODEL);
            Ok(Box::new(AnthropicClient::from_env(model, api_url)?))
        }
        "openai" | "local" | "ollama" => {
            let model = model.unwrap_or(OpenAiClient::DEFAULT_MODEL);
            Ok(Box::new(OpenAiClient::from_env(model, api_url)))
        }
        other => Err(JudgeError(format!(
            "Unknown --provider '{other}' (expected anthropic, openai, local, or ollama)"
        ))),
    }
}

/// Resolves the model id [`build_judge`] would use, WITHOUT constructing a client or requiring
/// credentials — used by the `--dry-run` path so its cache key matches what a real (non-dry-run)
/// run would compute (Java's dry-run consults the cache the same way). Mirrors `build_judge`'s
/// provider normalization + per-provider default-model fallback exactly.
pub fn resolve_model(provider: &str, model: Option<&str>) -> Result<String, JudgeError> {
    match provider.to_ascii_lowercase().as_str() {
        "anthropic" => Ok(model.unwrap_or(AnthropicClient::DEFAULT_MODEL).to_string()),
        "openai" | "local" | "ollama" => {
            Ok(model.unwrap_or(OpenAiClient::DEFAULT_MODEL).to_string())
        }
        other => Err(JudgeError(format!(
            "Unknown --provider '{other}' (expected anthropic, openai, local, or ollama)"
        ))),
    }
}

// ---------------------------------------------------------------------------------------
// env helpers — shared by both clients' `from_env`
// ---------------------------------------------------------------------------------------

/// Java `AnthropicClient`/`OpenAiClient`'s private `blankToNull(System.getenv(key))`, combined:
/// `None` if the variable is unset OR present-but-blank.
fn env_non_blank(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

/// Java `AnthropicClient`/`OpenAiClient`'s private `envOr(String, String)`.
fn env_or(key: &str, fallback: &str) -> String {
    env_non_blank(key).unwrap_or_else(|| fallback.to_string())
}

/// Java `AnthropicClient`/`OpenAiClient`'s private `trimTrailingSlash(String)`.
fn trim_trailing_slash(s: &str) -> &str {
    s.strip_suffix('/').unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- judge_max_tokens ----

    #[test]
    fn judge_max_tokens_uses_the_documented_formula_below_the_cap() {
        assert_eq!(judge_max_tokens(25), 2000 + 25 * 400); // 12000, the --batch default
        assert_eq!(judge_max_tokens(1), 2400);
        assert_eq!(judge_max_tokens(0), 2000);
    }

    #[test]
    fn judge_max_tokens_is_capped_at_32000() {
        // 2000 + 75*400 == 32000 exactly: the boundary where the cap first binds.
        assert_eq!(judge_max_tokens(75), 32000);
        assert_eq!(judge_max_tokens(76), 32000, "76 would be 32400 uncapped");
        assert_eq!(judge_max_tokens(1000), 32000);
    }

    // ---- retry_decision ----

    #[test]
    fn retry_decision_uses_exponential_backoff_keyed_on_the_current_attempt() {
        assert_eq!(
            retry_decision(429, 1, None),
            Some(Duration::from_millis(1000))
        );
        assert_eq!(
            retry_decision(500, 2, None),
            Some(Duration::from_millis(2000))
        );
        assert_eq!(
            retry_decision(503, 3, None),
            Some(Duration::from_millis(4000))
        );
    }

    #[test]
    fn retry_decision_gives_up_once_max_attempts_is_reached() {
        assert_eq!(retry_decision(429, MAX_JUDGE_ATTEMPTS, None), None);
        assert_eq!(retry_decision(500, MAX_JUDGE_ATTEMPTS, None), None);
        assert_eq!(retry_decision(429, MAX_JUDGE_ATTEMPTS + 1, None), None);
    }

    #[test]
    fn retry_decision_never_retries_a_non_retryable_client_error() {
        for status in [400, 401, 403, 404, 422] {
            assert_eq!(
                retry_decision(status, 1, None),
                None,
                "status {status} must not retry"
            );
        }
    }

    #[test]
    fn retry_decision_boundary_499_does_not_retry_but_500_does() {
        assert_eq!(retry_decision(499, 1, None), None);
        assert!(retry_decision(500, 1, None).is_some());
    }

    #[test]
    fn retry_decision_honors_a_positive_retry_after_hint_over_the_backoff_formula() {
        assert_eq!(
            retry_decision(429, 1, Some(7)),
            Some(Duration::from_secs(7))
        );
        assert_eq!(
            retry_decision(503, 1, Some(120)),
            Some(Duration::from_secs(120))
        );
    }

    #[test]
    fn retry_decision_falls_back_to_the_formula_when_retry_after_is_not_positive() {
        assert_eq!(
            retry_decision(429, 1, Some(0)),
            Some(Duration::from_millis(1000))
        );
    }

    #[test]
    fn retry_decision_ignores_retry_after_once_max_attempts_is_reached() {
        assert_eq!(retry_decision(429, MAX_JUDGE_ATTEMPTS, Some(5)), None);
    }

    #[test]
    fn parse_retry_after_secs_accepts_a_positive_integer_with_surrounding_whitespace() {
        assert_eq!(parse_retry_after_secs("7"), Some(7));
        assert_eq!(parse_retry_after_secs(" 30 "), Some(30));
    }

    #[test]
    fn parse_retry_after_secs_rejects_zero_negative_and_garbage() {
        assert_eq!(parse_retry_after_secs("0"), None);
        assert_eq!(parse_retry_after_secs("-5"), None);
        assert_eq!(parse_retry_after_secs("soon"), None);
        assert_eq!(parse_retry_after_secs(""), None);
    }

    // ---- Anthropic response extraction — ported from AnthropicClientTest ----

    #[test]
    fn extract_anthropic_verdicts_from_a_canned_messages_response() {
        // Ported from `AnthropicClientTest.parsesStructuredVerdicts`: a representative Messages
        // response — an (empty) thinking block (must be skipped), then a text block whose
        // content is the structured-output JSON.
        let inner_json = concat!(
            "{\"verdicts\":[",
            "{\"index\":0,\"verdict\":\"ok\",\"confidence\":\"high\",\"fields\":[],\"note\":\"\"},",
            "{\"index\":1,\"verdict\":\"wrong\",\"confidence\":\"med\",",
            "\"fields\":[{\"name\":\"rank\",\"parsed\":\"INFRASPECIFIC_NAME\",",
            "\"expected\":\"SUBSPECIES\",\"reason\":\"zoological trinomial\"}],\"note\":\"x\"}",
            "]}",
        );
        let response = serde_json::json!({
            "content": [
                {"type": "thinking", "thinking": ""},
                {"type": "text", "text": inner_json},
            ]
        })
        .to_string();

        let verdicts = extract_anthropic_verdicts(&response).expect("must extract + parse");
        assert_eq!(verdicts.len(), 2);

        let ok = &verdicts[0];
        assert_eq!(ok.index, 0);
        assert!(ok.is_ok());

        let wrong = &verdicts[1];
        assert_eq!(wrong.index, 1);
        assert_eq!(wrong.verdict, "wrong");
        assert_eq!(wrong.fields.len(), 1);
        assert_eq!(wrong.fields[0].name, "rank");
        assert_eq!(wrong.fields[0].expected, "SUBSPECIES");
    }

    #[test]
    fn extract_anthropic_verdicts_concatenates_multiple_text_blocks() {
        // Not itself a Java test case, but pins the documented "concatenate the text of EVERY
        // text block" behavior (plural), not just "use the first one" — split the same JSON
        // across two adjacent text blocks.
        let response = serde_json::json!({
            "content": [
                {"type": "text", "text": "{\"verdicts\":[{\"index\":0,\"verdict\":\"ok\","},
                {"type": "text", "text": "\"confidence\":\"high\",\"fields\":[],\"note\":\"\"}]}"},
            ]
        })
        .to_string();
        let verdicts = extract_anthropic_verdicts(&response).expect("must concatenate + parse");
        assert_eq!(verdicts.len(), 1);
        assert!(verdicts[0].is_ok());
    }

    #[test]
    fn extract_anthropic_verdicts_errors_when_no_text_block_is_present() {
        let response =
            serde_json::json!({"content": [{"type": "thinking", "thinking": "..."}]}).to_string();
        let err = extract_anthropic_verdicts(&response).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn anthropic_verdict_schema_matches_the_documented_shape() {
        // Ported from `AnthropicClientTest.schemaIsWellFormed`, plus stronger structural
        // assertions this port's exact recon transcription affords.
        let schema = verdict_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["verdicts"].is_object());
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["required"], serde_json::json!(["verdicts"]));

        let verdict_items = &schema["properties"]["verdicts"]["items"];
        assert_eq!(verdict_items["type"], "object");
        assert_eq!(verdict_items["additionalProperties"], false);
        let required = verdict_items["required"].as_array().unwrap();
        for field in ["index", "verdict", "confidence", "fields", "note"] {
            assert!(
                required.iter().any(|v| v == field),
                "verdict schema must require {field}"
            );
        }
        assert_eq!(
            verdict_items["properties"]["verdict"]["enum"],
            serde_json::json!(["ok", "suspect", "wrong"])
        );
        assert_eq!(
            verdict_items["properties"]["confidence"]["enum"],
            serde_json::json!(["low", "med", "high"])
        );

        let issue_items = &verdict_items["properties"]["fields"]["items"];
        assert_eq!(issue_items["additionalProperties"], false);
        let issue_required = issue_items["required"].as_array().unwrap();
        for field in ["name", "parsed", "expected", "reason"] {
            assert!(
                issue_required.iter().any(|v| v == field),
                "field-issue schema must require {field}"
            );
            assert_eq!(issue_items["properties"][field]["type"], "string");
        }
    }

    // ---- OpenAI-compatible response extraction — ported from OpenAiClientTest ----

    #[test]
    fn extract_openai_content_from_a_canned_chat_completion() {
        // Ported from `OpenAiClientTest.extractsChatCompletionContent`.
        let resp = serde_json::json!({
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "the-content"}}]
        })
        .to_string();
        assert_eq!(extract_openai_content(&resp).unwrap(), "the-content");
    }

    #[test]
    fn extract_openai_content_errors_when_choices_is_missing_or_empty() {
        assert_eq!(
            extract_openai_content("{}").unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        assert_eq!(
            extract_openai_content("{\"choices\":[]}")
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn extract_openai_content_errors_when_message_content_is_absent() {
        let resp = serde_json::json!({"choices": [{"message": {"role": "assistant"}}]}).to_string();
        assert_eq!(
            extract_openai_content(&resp).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn openai_finish_reason_detects_truncated_and_stop() {
        // Ported from `OpenAiClientTest.detectsTruncatedFinishReason`.
        let truncated = serde_json::json!({
            "choices": [{"index": 0, "finish_reason": "length",
                         "message": {"role": "assistant", "content": "..."}}]
        })
        .to_string();
        assert_eq!(openai_finish_reason(&truncated).as_deref(), Some("length"));

        let complete = serde_json::json!({
            "choices": [{"index": 0, "finish_reason": "stop",
                         "message": {"role": "assistant", "content": "..."}}]
        })
        .to_string();
        assert_eq!(openai_finish_reason(&complete).as_deref(), Some("stop"));
    }

    #[test]
    fn openai_finish_reason_is_none_when_unparseable_or_absent() {
        assert_eq!(openai_finish_reason("not json"), None);
        assert_eq!(openai_finish_reason("{}"), None);
        assert_eq!(openai_finish_reason("{\"choices\":[]}"), None);
    }

    #[test]
    fn parse_openai_reply_degrades_to_empty_on_unusable_batch_instead_of_erroring() {
        // Ported from `OpenAiClientTest.parseReplySkipsUnusableBatchInsteadOfThrowing`: empty
        // content (with a truncation finish_reason) and a reply with no verdicts array both
        // degrade to an empty (unjudged) batch rather than propagating an error.
        let empty = serde_json::json!({
            "choices": [{"finish_reason": "length",
                         "message": {"role": "assistant", "content": ""}}]
        })
        .to_string();
        assert!(parse_openai_reply(&empty, 10, "test-model").is_empty());

        let no_verdicts = serde_json::json!({
            "choices": [{"finish_reason": "stop",
                         "message": {"role": "assistant",
                                     "content": "I could not produce JSON, sorry."}}]
        })
        .to_string();
        assert!(parse_openai_reply(&no_verdicts, 10, "test-model").is_empty());
    }

    #[test]
    fn parse_openai_reply_returns_verdicts_for_a_good_batch() {
        // Ported from `OpenAiClientTest.parseReplyReturnsVerdictsForGoodBatch`.
        let inner = "{\"verdicts\":[{\"index\":0,\"verdict\":\"ok\",\"confidence\":\"high\",\"fields\":[]}]}";
        let good = serde_json::json!({
            "choices": [{"finish_reason": "stop",
                         "message": {"role": "assistant", "content": inner}}]
        })
        .to_string();
        let verdicts = parse_openai_reply(&good, 10, "test-model");
        assert_eq!(verdicts.len(), 1);
        assert!(verdicts[0].is_ok());
    }

    #[test]
    fn parse_openai_reply_skips_a_malformed_element_but_keeps_the_rest() {
        // A batch of 3 where the middle element is malformed now yields 2 verdicts, not an
        // empty/degraded batch (Task 4's Step-0 fix threading all the way through the OpenAI
        // response path, not just the bare `parse_verdicts` function).
        let inner = concat!(
            "{\"verdicts\":[",
            "{\"index\":0,\"verdict\":\"ok\",\"confidence\":\"high\",\"fields\":[],\"note\":\"\"},",
            "{\"index\":1,\"confidence\":\"high\",\"fields\":[]},",
            "{\"index\":2,\"verdict\":\"suspect\",\"confidence\":\"low\",\"fields\":[],\"note\":\"\"}",
            "]}",
        );
        let resp = serde_json::json!({
            "choices": [{"finish_reason": "stop",
                         "message": {"role": "assistant", "content": inner}}]
        })
        .to_string();
        let verdicts = parse_openai_reply(&resp, 10, "test-model");
        assert_eq!(verdicts.len(), 2);
        assert_eq!(verdicts[0].index, 0);
        assert_eq!(verdicts[1].index, 2);
    }

    // ---- build_judge — provider normalization + default-model resolution ----
    //
    // Deliberately does NOT exercise `build_judge("anthropic", ...)`: `AnthropicClient::from_env`
    // reads real process environment variables (`ANTHROPIC_API_KEY`/`ANTHROPIC_AUTH_TOKEN`) and
    // its success/failure is genuinely environment-dependent (unlike Java's own
    // `AnthropicClientTest`/`OpenAiClientTest`, which also never test `fromEnv`'s env-var
    // resolution) — asserting either outcome here would be flaky on any machine that happens to
    // have (or not have) those variables set, so this is left untested by design, same as Java.

    #[test]
    fn build_judge_normalizes_openai_local_and_ollama_to_the_openai_default_model() {
        for provider in ["openai", "local", "ollama", "LOCAL", "Ollama"] {
            let judge = build_judge(provider, None, Some("http://example.invalid"))
                .unwrap_or_else(|e| panic!("provider {provider:?} must build: {e}"));
            assert_eq!(judge.model_id(), OpenAiClient::DEFAULT_MODEL);
        }
    }

    #[test]
    fn build_judge_honors_an_explicit_model_override() {
        let judge = build_judge(
            "local",
            Some("custom-model"),
            Some("http://example.invalid"),
        )
        .expect("must build");
        assert_eq!(judge.model_id(), "custom-model");
    }

    #[test]
    fn build_judge_rejects_an_unknown_provider() {
        // `Box<dyn Judge>` isn't `Debug` (the trait has no such bound), so `unwrap_err()`
        // (which requires the `Ok` side to be `Debug`) doesn't work here — match instead.
        let err = match build_judge("bogus", None, None) {
            Err(e) => e,
            Ok(_) => panic!("an unknown provider must not build a Judge"),
        };
        assert!(err.to_string().contains("bogus"));
    }

    #[test]
    fn openai_client_endpoint_appends_the_chat_completions_path() {
        // `OpenAiClient::from_env` needs no credential, so — unlike `AnthropicClient::from_env`
        // — this is safe to construct directly with an explicit `api_url` and no environment
        // dependency at all (`endpoint` is a private field, visible here since `tests` is a
        // child module of `client`).
        let direct = OpenAiClient::from_env("m", Some("http://example.invalid/"));
        assert_eq!(
            direct.endpoint,
            "http://example.invalid/v1/chat/completions"
        );
    }

    // Deliberately no equivalent direct-construction test for `AnthropicClient::from_env`'s
    // endpoint-joining: doing so would need either a real credential env var (flaky/unsafe to
    // mutate in a parallel-test process, see the `build_judge` note above) or a second,
    // untested code path just for tests. `AnthropicClient::from_env` and `OpenAiClient::from_env`
    // share the identical `format!("{}/vX/...", trim_trailing_slash(&base))` pattern (see both
    // `from_env` bodies) and the same `trim_trailing_slash` helper (tested below on its own) —
    // the OpenAI test above exercises that shared logic without the credential hazard.

    // ---- trim_trailing_slash / env helpers ----

    #[test]
    fn trim_trailing_slash_removes_at_most_one_trailing_slash() {
        assert_eq!(trim_trailing_slash("http://x/"), "http://x");
        assert_eq!(trim_trailing_slash("http://x"), "http://x");
        assert_eq!(trim_trailing_slash("http://x//"), "http://x/");
    }
}
