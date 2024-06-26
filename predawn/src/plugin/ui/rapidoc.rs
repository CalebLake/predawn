use std::sync::Arc;

use http::Method;
use indexmap::IndexMap;
use rudi::{Context, Singleton};

use crate::{config::Config, handler::DynHandler, normalized_path::NormalizedPath, plugin::Plugin};

const TEMPLATE: &str = r###"
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="description" content="{{description}}" />
    <title>{{title}}</title>
    <script type="module" src="{{js_url}}"></script>
  </head>
  <body>
    <rapi-doc spec-url="{{spec_url}}"> </rapi-doc>
  </body>
</html>
"###;

#[derive(Clone, Debug)]
pub struct RapiDoc {
    description: Box<str>,
    title: Box<str>,
    js_url: Box<str>,
    spec_url: Box<str>,
}

impl Plugin for RapiDoc {
    fn create_route(
        self: Arc<Self>,
        cx: &mut Context,
    ) -> (NormalizedPath, IndexMap<Method, DynHandler>) {
        super::create_route(cx, |c| c.rapidoc_path, self.as_html())
    }
}

fn condition(cx: &Context) -> bool {
    !cx.contains_provider::<RapiDoc>()
}

#[Singleton(condition = condition)]
fn RapiDocRegister(#[di(ref)] cfg: &Config) -> RapiDoc {
    let json_path = super::json_path(cfg).into_inner();
    RapiDoc::new(json_path)
}

#[Singleton(name = std::any::type_name::<RapiDoc>())]
fn RapiDocToPlugin(rapidoc: RapiDoc) -> Arc<dyn Plugin> {
    Arc::new(rapidoc)
}

impl RapiDoc {
    pub fn new<T>(spec_url: T) -> Self
    where
        T: Into<Box<str>>,
    {
        Self {
            description: Box::from("RapiDoc"),
            title: Box::from("RapiDoc"),
            js_url: Box::from("https://unpkg.com/rapidoc/dist/rapidoc-min.js"),
            spec_url: spec_url.into(),
        }
    }

    pub fn as_html(&self) -> String {
        TEMPLATE
            .replacen("{{description}}", &self.description, 1)
            .replacen("{{title}}", &self.title, 1)
            .replacen("{{js_url}}", &self.js_url, 1)
            .replacen("{{spec_url}}", &self.spec_url, 1)
    }
}
