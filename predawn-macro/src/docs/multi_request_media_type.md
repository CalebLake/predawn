Define a single request body with multiple media types.

This macro will generate 3 implementations, [`MultiRequestMediaType`], [`FromRequest`] and [`ApiRequest`].

## Example

```rust
use predawn::{
    define_from_request_error,
    payload::{Form, Json},
    response_error::{ReadFormError, ReadJsonError},
    MultiRequestMediaType, ToSchema,
};
use serde::de::DeserializeOwned;

#[derive(Debug, MultiRequestMediaType)]
#[multi_request_media_type(error = ReadJsonOrFormError)]
pub enum JsonOrForm<T: ToSchema + DeserializeOwned> {
    Json(Json<T>),
    Form(Form<T>),
}

define_from_request_error! {
    name: ReadJsonOrFormError,
    errors: [
        ReadJsonError,
        ReadFormError,
    ],
}
```

[`MultiRequestMediaType`]: https://docs.rs/predawn/latest/predawn/trait.MultiRequestMediaType.html
[`FromRequest`]: https://docs.rs/predawn/latest/predawn/from_request/trait.FromRequest.html
[`ApiRequest`]: https://docs.rs/predawn/latest/predawn/api_request/trait.ApiRequest.html
