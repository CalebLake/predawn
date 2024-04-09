use std::{collections::BTreeMap, convert::Infallible};

use bytes::Bytes;
use http::{header::CONTENT_TYPE, HeaderValue, StatusCode};
use mime::{APPLICATION, WWW_FORM_URLENCODED};
use predawn_core::{
    api_request::ApiRequest,
    api_response::ApiResponse,
    body::RequestBody,
    from_request::FromRequest,
    impl_deref,
    into_response::IntoResponse,
    media_type::{
        has_media_type, MediaType, MultiRequestMediaType, MultiResponseMediaType, RequestMediaType,
        ResponseMediaType, SingleMediaType,
    },
    openapi::{self, Components, Parameter},
    request::Head,
    response::{MultiResponse, Response, SingleResponse},
};
use predawn_schema::ToSchema;
use serde::{de::DeserializeOwned, Serialize};

use crate::response_error::{ReadFormError, WriteFormError};

#[derive(Debug, Default, Clone, Copy)]
pub struct Form<T>(pub T);

impl_deref!(Form);

impl<'a, T> FromRequest<'a> for Form<T>
where
    T: DeserializeOwned,
{
    type Error = ReadFormError;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        let content_type = head.content_type().unwrap_or_default();

        if <Self as RequestMediaType>::check_content_type(content_type) {
            let bytes = Bytes::from_request(head, body).await?;

            match serde_html_form::from_bytes::<T>(&bytes) {
                Ok(value) => Ok(Form(value)),
                Err(err) => Err(ReadFormError::FormDeserializeError(err)),
            }
        } else {
            Err(ReadFormError::InvalidFormContentType)
        }
    }
}

impl<T: ToSchema> ApiRequest for Form<T> {
    fn parameters(_: &mut Components) -> Option<Vec<Parameter>> {
        None
    }

    fn request_body(components: &mut Components) -> Option<openapi::RequestBody> {
        Some(openapi::RequestBody {
            content: <Self as MultiRequestMediaType>::content(components),
            required: true,
            ..Default::default()
        })
    }
}

impl<T> IntoResponse for Form<T>
where
    T: Serialize,
{
    type Error = WriteFormError;

    fn into_response(self) -> Result<Response, Self::Error> {
        let mut response = serde_html_form::to_string(&self.0)
            .map_err(WriteFormError)?
            .into_response()
            .unwrap_or_else(|a: Infallible| match a {});

        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static(<Self as MediaType>::MEDIA_TYPE),
        );

        Ok(response)
    }
}

impl<T: ToSchema> ApiResponse for Form<T> {
    fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        Some(<Self as MultiResponse>::responses(components))
    }
}

impl<T> MediaType for Form<T> {
    const MEDIA_TYPE: &'static str = "application/x-www-form-urlencoded";
}

impl<T> RequestMediaType for Form<T> {
    fn check_content_type(content_type: &str) -> bool {
        has_media_type(
            content_type,
            APPLICATION.as_str(),
            WWW_FORM_URLENCODED.as_str(),
            WWW_FORM_URLENCODED.as_str(),
            None,
        )
    }
}

impl<T> ResponseMediaType for Form<T> {}

impl<T: ToSchema> SingleMediaType for Form<T> {
    fn media_type(components: &mut Components) -> openapi::MediaType {
        openapi::MediaType {
            schema: Some(T::schema_ref(components)),
            ..Default::default()
        }
    }
}

impl<T: ToSchema> SingleResponse for Form<T> {
    fn response(components: &mut Components) -> openapi::Response {
        openapi::Response {
            description: "FORM response".to_owned(),
            content: <Self as MultiResponseMediaType>::content(components),
            ..Default::default()
        }
    }
}
