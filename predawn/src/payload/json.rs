use std::{collections::BTreeMap, convert::Infallible};

use bytes::{BufMut, Bytes, BytesMut};
use http::{
    header::{HeaderValue, CONTENT_TYPE},
    StatusCode,
};
use mime::{APPLICATION, JSON};
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
use serde_json::error::Category;

use crate::response_error::{ReadJsonError, WriteJsonError};

#[derive(Debug, Default, Clone, Copy)]
pub struct Json<T>(pub T);

impl_deref!(Json);

impl<'a, T> FromRequest<'a> for Json<T>
where
    T: DeserializeOwned,
{
    type Error = ReadJsonError;

    async fn from_request(head: &'a Head, body: RequestBody) -> Result<Self, Self::Error> {
        let content_type = head.content_type().unwrap_or_default();

        if <Self as RequestMediaType>::check_content_type(content_type) {
            let bytes = Bytes::from_request(head, body).await?;
            Self::from_bytes(&bytes)
        } else {
            Err(ReadJsonError::InvalidJsonContentType)
        }
    }
}

impl<T: ToSchema> ApiRequest for Json<T> {
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

impl<T> Json<T>
where
    T: DeserializeOwned,
{
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReadJsonError> {
        let deserializer = &mut serde_json::Deserializer::from_slice(bytes);

        match serde_path_to_error::deserialize(deserializer) {
            Ok(value) => Ok(Json(value)),
            Err(err) => {
                let error = match err.inner().classify() {
                    Category::Data => ReadJsonError::JsonDataError(err),
                    Category::Syntax | Category::Eof => ReadJsonError::JsonSyntaxError(err),
                    Category::Io => {
                        if cfg!(debug_assertions) {
                            // we don't use `serde_json::from_reader` and instead always buffer
                            // bodies first, so we shouldn't encounter any IO errors
                            unreachable!()
                        } else {
                            ReadJsonError::JsonSyntaxError(err)
                        }
                    }
                };
                Err(error)
            }
        }
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    type Error = WriteJsonError;

    fn into_response(self) -> Result<Response, Self::Error> {
        let mut buf = BytesMut::with_capacity(128).writer();

        match serde_json::to_writer(&mut buf, &self.0) {
            Ok(_) => {
                let mut response = buf
                    .into_inner()
                    .into_response()
                    .unwrap_or_else(|a: Infallible| match a {});

                response.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static(<Self as MediaType>::MEDIA_TYPE),
                );

                Ok(response)
            }
            Err(err) => Err(WriteJsonError(err)),
        }
    }
}

impl<T: ToSchema> ApiResponse for Json<T> {
    fn responses(components: &mut Components) -> Option<BTreeMap<StatusCode, openapi::Response>> {
        Some(<Self as MultiResponse>::responses(components))
    }
}

impl<T> MediaType for Json<T> {
    const MEDIA_TYPE: &'static str = "application/json";
}

impl<T> RequestMediaType for Json<T> {
    fn check_content_type(content_type: &str) -> bool {
        has_media_type(
            content_type,
            APPLICATION.as_str(),
            JSON.as_str(),
            JSON.as_str(),
            None,
        )
    }
}

impl<T> ResponseMediaType for Json<T> {}

impl<T: ToSchema> SingleMediaType for Json<T> {
    fn media_type(components: &mut Components) -> openapi::MediaType {
        openapi::MediaType {
            schema: Some(T::schema_ref(components)),
            ..Default::default()
        }
    }
}

impl<T: ToSchema> SingleResponse for Json<T> {
    fn response(components: &mut Components) -> openapi::Response {
        openapi::Response {
            description: "JSON response".to_owned(),
            content: <Self as MultiResponseMediaType>::content(components),
            ..Default::default()
        }
    }
}
