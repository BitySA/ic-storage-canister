use candid::{define_function, CandidType, Deserialize};
use ic_http_certification::{HeaderField, HttpRequest, HttpResponse, StatusCode};
use serde_bytes::ByteBuf;

pub type Args = HttpRequest<'static>;
pub type Response = HttpStreamingResponse;

/// Byte offset of the next slice to send, together with the file it belongs to.
///
/// The HTTP gateway hands this back verbatim on every callback, so it is the
/// only state the streaming protocol carries between calls.
#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StreamingCallbackToken {
    pub path: String,
    pub index: u64,
    pub content_type: String,
    pub total_length: u64,
}

define_function!(pub CallbackFunc : (StreamingCallbackToken) -> (StreamingCallbackHttpResponse) query);

#[derive(CandidType, Deserialize, Clone, Debug)]
pub enum StreamingStrategy {
    Callback {
        callback: CallbackFunc,
        token: StreamingCallbackToken,
    },
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct StreamingCallbackHttpResponse {
    pub body: ByteBuf,
    /// `None` ends the stream.
    pub token: Option<StreamingCallbackToken>,
}

/// The response shape the HTTP gateway expects.
///
/// `ic_http_certification::HttpResponse` cannot express `streaming_strategy`,
/// which is the only supported way to return a body larger than the 3 MiB
/// message cap, so the canister returns this type instead and converts the
/// certification crate's responses into it.
#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct HttpStreamingResponse {
    pub status_code: u16,
    pub headers: Vec<HeaderField>,
    pub body: ByteBuf,
    pub upgrade: Option<bool>,
    pub streaming_strategy: Option<StreamingStrategy>,
}

impl HttpStreamingResponse {
    pub fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// The bytes carried by this response. For a streamed response this is the
    /// first chunk only; the rest arrives through the streaming callback.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn headers(&self) -> &[HeaderField] {
        &self.headers
    }
}

impl From<HttpResponse<'_>> for HttpStreamingResponse {
    fn from(response: HttpResponse<'_>) -> Self {
        Self {
            status_code: response.status_code().as_u16(),
            headers: response.headers().to_vec(),
            body: ByteBuf::from(response.body().to_vec()),
            upgrade: response.upgrade(),
            streaming_strategy: None,
        }
    }
}
