use {
    std::mem,
    rocket::{
        form,
        http::uri::Origin,
        response::content::RawHtml,
    },
    rocket_csrf::CsrfToken,
    rocket_util::{
        ToHtml,
        html,
    },
};

fn render_form_error(error: &form::Error<'_>) -> RawHtml<String> {
    html! {
        div(class = "alert alert-danger col-sm-10 col-sm-offset-2", role = "alert") {
            span(class = "glyphicon glyphicon-exclamation-sign", aria_hidden = "true");
            : " ";
            span(class = "sr-only") : "Error:";
            : error;
        }
    }
}

pub(crate) fn form_field(name: &str, errors: &mut Vec<&form::Error<'_>>, label: impl ToHtml, content: impl ToHtml, description: Option<RawHtml<String>>) -> RawHtml<String> {
    let field_errors;
    (field_errors, *errors) = mem::take(errors).into_iter().partition(|error| error.is_for(name));
    html! {
        @for error in field_errors {
            : render_form_error(error);
        }
        div(class = "form-group") {
            label(class = "col-sm-2 control-label", for = "input_description") {
                label(for = name) : label;
            }
            div(class = "col-sm-10") : content;
            @if let Some(description) = description {
                div(class = "col-sm-10 col-sm-offset-2") {
                    span(class = "muted") : description;
                }
            }
        }
    }
}

pub(crate) fn form_checkbox(name: &str, errors: &mut Vec<&form::Error<'_>>, label: impl ToHtml, checked: bool, description: Option<RawHtml<String>>) -> RawHtml<String> {
    let field_errors;
    (field_errors, *errors) = mem::take(errors).into_iter().partition(|error| error.is_for(name));
    html! {
        @for error in field_errors {
            : render_form_error(error);
        }
        div(class = "form-group") {
            div(class = "col-sm-offset-2 col-sm-10") {
                div(class = "checkbox") {
                    label {
                        input(checked? = checked, name = name, type = "checkbox");
                        label(for = name) : label;
                    }
                }
            }
            @if let Some(description) = description {
                div(class = "col-sm-10 col-sm-offset-2") {
                    span(class = "muted") : description;
                }
            }
        }
    }
}

pub(crate) fn full_form(uri: Origin<'_>, csrf: Option<&CsrfToken>, content: impl ToHtml, errors: Vec<&form::Error<'_>>, submit_text: &str) -> RawHtml<String> {
    html! {
        form(class = "form-horizontal", role = "form", action = uri, method = "post") {
            : csrf;
            @for error in errors {
                : render_form_error(error);
            }
            : content;
            div(class = "form-group") {
                div(class = "col-sm-10 col-sm-offset-2") {
                    button(class = "btn btn-primary", type = "submit", value = submit_text) : submit_text;
                }
            }
        }
    }
}
