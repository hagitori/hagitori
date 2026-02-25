//! QuickJS HTML parsing API (DOM selection, text extraction via `scraper`).

use rquickjs::{Array, Ctx, Function, Object, Value};
use rquickjs::prelude::This;
use scraper::{Html, Selector};

pub fn register<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    // parseHtml(htmlString) -> Document
    globals.set(
        "parseHtml",
        Function::new(ctx.clone(), |ctx: Ctx<'js>, body: String| {
            let html = Html::parse_document(&body);
            create_document_object(&ctx, &html)
        })?,
    )?;

    Ok(())
}

struct ElementData {
    text: String,
    inner_html: String,
    outer_html: String,
    attrs: Vec<(String, String)>,
}

impl ElementData {
    fn from_element(element: &scraper::ElementRef) -> Self {
        Self {
            text: element.text().collect::<String>(),
            inner_html: element.inner_html(),
            outer_html: element.html(),
            attrs: element
                .value()
                .attrs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }
}

fn create_document_object<'js>(ctx: &Ctx<'js>, html: &Html) -> rquickjs::Result<Object<'js>> {
    let doc = Object::new(ctx.clone())?;

    let html_string = html.html();
    doc.set("_html", html_string)?;

    // select(css) -> Element[]
    doc.set(
        "select",
        Function::new(ctx.clone(), |ctx: Ctx<'js>, this: This<Object<'js>>, selector_str: String| {
            let html_str: String = this.0.get("_html")?;
            let html = Html::parse_document(&html_str);
            let selector = Selector::parse(&selector_str)
                .map_err(|e| rquickjs::Error::new_from_js_message("string", "selector", &format!("invalid CSS selector '{selector_str}': {e}")))?;

            let arr = Array::new(ctx.clone())?;
            for (i, element) in html.select(&selector).enumerate() {
                let elem_data = ElementData::from_element(&element);
                let el_obj = create_element_object(&ctx, &elem_data)?;
                arr.set(i, el_obj)?;
            }
            Ok::<_, rquickjs::Error>(arr)
        })?,
    )?;

    // selectOne(css) -> Element | null
    doc.set(
        "selectOne",
        Function::new(ctx.clone(), |ctx: Ctx<'js>, this: This<Object<'js>>, selector_str: String| {
            let html_str: String = this.0.get("_html")?;
            let html = Html::parse_document(&html_str);
            let selector = Selector::parse(&selector_str)
                .map_err(|e| rquickjs::Error::new_from_js_message("string", "selector", &format!("invalid CSS selector '{selector_str}': {e}")))?;

            match html.select(&selector).next() {
                Some(element) => {
                    let elem_data = ElementData::from_element(&element);
                    let el_obj = create_element_object(&ctx, &elem_data)?;
                    Ok::<_, rquickjs::Error>(el_obj.into())
                }
                None => Ok::<_, rquickjs::Error>(Value::new_null(ctx)),
            }
        })?,
    )?;

    // text() -> string
    doc.set(
        "text",
        Function::new(ctx.clone(), |this: This<Object<'js>>| {
            let html_str: String = this.0.get("_html")?;
            let html = Html::parse_document(&html_str);
            Ok::<_, rquickjs::Error>(html.root_element().text().collect::<String>())
        })?,
    )?;

    // html() -> string
    doc.set(
        "html",
        Function::new(ctx.clone(), |this: This<Object<'js>>| {
            let html_str: String = this.0.get("_html")?;
            Ok::<_, rquickjs::Error>(html_str)
        })?,
    )?;

    Ok(doc)
}

fn create_element_object<'js>(ctx: &Ctx<'js>, data: &ElementData) -> rquickjs::Result<Object<'js>> {
    let element = Object::new(ctx.clone())?;

    element.set("_text", data.text.clone())?;
    element.set("_html", data.inner_html.clone())?;
    element.set("_outerHtml", data.outer_html.clone())?;

    let attrs_obj = Object::new(ctx.clone())?;
    for (key, value) in &data.attrs {
        attrs_obj.set(key.as_str(), value.clone())?;
    }
    element.set("_attrs", attrs_obj)?;

    // text() -> string
    element.set(
        "text",
        Function::new(ctx.clone(), |this: This<Object<'js>>| {
            let text: String = this.0.get("_text")?;
            Ok::<_, rquickjs::Error>(text)
        })?,
    )?;

    // attr(name) -> string | null
    element.set(
        "attr",
        Function::new(ctx.clone(), |this: This<Object<'js>>, name: String| {
            let attrs: Object = this.0.get("_attrs")?;
            let val: Value = attrs.get(name.as_str())?;
            if val.is_undefined() {
                Ok::<_, rquickjs::Error>(Value::new_null(this.0.ctx().clone()))
            } else {
                Ok::<_, rquickjs::Error>(val)
            }
        })?,
    )?;

    // html() -> string
    element.set(
        "html",
        Function::new(ctx.clone(), |this: This<Object<'js>>| {
            let html: String = this.0.get("_html")?;
            Ok::<_, rquickjs::Error>(html)
        })?,
    )?;

    // outerHtml() -> string
    element.set(
        "outerHtml",
        Function::new(ctx.clone(), |this: This<Object<'js>>| {
            let html: String = this.0.get("_outerHtml")?;
            Ok::<_, rquickjs::Error>(html)
        })?,
    )?;

    // select(css) -> Element[]
    element.set(
        "select",
        Function::new(ctx.clone(), |ctx: Ctx<'js>, this: This<Object<'js>>, selector_str: String| {
            let inner_html: String = this.0.get("_html")?;
            let fragment = Html::parse_fragment(&inner_html);
            let selector = Selector::parse(&selector_str)
                .map_err(|e| rquickjs::Error::new_from_js_message("string", "selector", &format!("invalid CSS selector: {e}")))?;

            let arr = Array::new(ctx.clone())?;
            for (i, element) in fragment.select(&selector).enumerate() {
                let elem_data = ElementData::from_element(&element);
                let el_obj = create_element_object(&ctx, &elem_data)?;
                arr.set(i, el_obj)?;
            }
            Ok::<_, rquickjs::Error>(arr)
        })?,
    )?;

    // selectOne(css) -> Element | null
    element.set(
        "selectOne",
        Function::new(ctx.clone(), |ctx: Ctx<'js>, this: This<Object<'js>>, selector_str: String| {
            let inner_html: String = this.0.get("_html")?;
            let fragment = Html::parse_fragment(&inner_html);
            let selector = Selector::parse(&selector_str)
                .map_err(|e| rquickjs::Error::new_from_js_message("string", "selector", &format!("invalid CSS selector: {e}")))?;

            match fragment.select(&selector).next() {
                Some(element) => {
                    let elem_data = ElementData::from_element(&element);
                    let el_obj = create_element_object(&ctx, &elem_data)?;
                    Ok::<_, rquickjs::Error>(el_obj.into())
                }
                None => Ok::<_, rquickjs::Error>(Value::new_null(ctx)),
            }
        })?,
    )?;

    Ok(element)
}
