use headers::HeaderValue;
use viz_core::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    types::{Form, Json, PayloadError, State, StateError},
    FromRequest, IncomingBody, Request, RequestExt, Result,
};

#[tokio::test]
async fn from_request() -> Result<()> {
    let mut req = Request::builder()
        .header(
            CONTENT_TYPE,
            HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
        )
        .header(CONTENT_LENGTH, "0")
        .body(IncomingBody::Empty)?;
    let result: Result<Json<Option<String>>, PayloadError> = FromRequest::extract(&mut req).await;
    assert!(result.is_err());

    let mut req = Request::builder()
        .header(
            CONTENT_TYPE,
            HeaderValue::from_static(mime::APPLICATION_WWW_FORM_URLENCODED.as_ref()),
        )
        .header(CONTENT_LENGTH, "0")
        .body(IncomingBody::Empty)?;
    let result: Result<Form<Option<String>>, PayloadError> = req.extract().await;
    assert!(result.is_err());

    let mut req = Request::builder().body(IncomingBody::Empty)?;
    let state: Option<State<String>> = req.extract().await?;
    assert!(state.is_none());

    let mut req = Request::builder().body(IncomingBody::Empty)?;
    let result: Result<State<String>, StateError> = req.extract().await?;
    assert!(result.is_err());

    Ok(())
}