use quote;
use validator::Validator;
use syn;

use lit::option_u64_to_tokens;
use validation::{FieldValidation, SchemaValidation};
use asserts::NUMBER_TYPES;


/// Pass around all the information needed for creating a validation
#[derive(Debug)]
pub struct FieldQuoter {
    ident: syn::Ident,
    /// The field name
    name: String,
    /// The field type
    _type: String,
}

impl FieldQuoter {
    pub fn new(ident: syn::Ident, name: String, _type: String) -> FieldQuoter {
        FieldQuoter { ident, name, _type }
    }

    /// Don't put a & in front a pointer since we are going to pass
    /// a reference to the validator
    /// Also just use the ident without if it's optional and will go through
    /// a if let first
    pub fn quote_validator_param(&self) -> quote::Tokens {
        let ident = &self.ident;

        if self._type.starts_with("Option<") {
            quote!(#ident)
        } else if self._type.starts_with("&") || NUMBER_TYPES.contains(&self._type.as_ref()) {
            quote!(self.#ident)
        } else {
            quote!(&self.#ident)
        }
    }

    pub fn get_optional_validator_param(&self) -> quote::Tokens {
        let ident = &self.ident;
        if self._type.starts_with("Option<&") || self._type.starts_with("Option<Option<&")
            || NUMBER_TYPES.contains(&self._type.as_ref()) {
          quote!(#ident)
        } else {
          quote!(ref #ident)
        }
    }

    /// Wrap the quoted output of a validation with a if let Some if
    /// the field type is an option
    pub fn wrap_if_option(&self, tokens: quote::Tokens) -> quote::Tokens {
        let field_ident = &self.ident;
        let optional_pattern_matched = self.get_optional_validator_param();
        if self._type.starts_with("Option<Option<") {
            return quote!(
                if let Some(Some(#optional_pattern_matched)) = self.#field_ident {
                    #tokens
                }
            )
        } else if self._type.starts_with("Option<") {
            return quote!(
                if let Some(#optional_pattern_matched) = self.#field_ident {
                    #tokens
                }
            )
        }

        tokens
    }
}

/// Quote an actual end-user error creation automatically
fn quote_error(validation: &FieldValidation) -> quote::Tokens {
    let code = &validation.code;
    let add_message_quoted = if let Some(ref m) = validation.message {
        quote!(err.message = Some(::std::borrow::Cow::from(#m));)
    } else {
        quote!()
    };

    quote!(
        let mut err = ::validator::ValidationError::new(#code);
        #add_message_quoted
    )
}


pub fn quote_length_validation(field_quoter: &FieldQuoter, validation: &FieldValidation) -> quote::Tokens {
    let field_name = &field_quoter.name;
    let validator_param = field_quoter.quote_validator_param();

    if let Validator::Length { min, max, equal } = validation.validator {
        // Can't interpolate None
        let min_tokens = option_u64_to_tokens(min);
        let max_tokens = option_u64_to_tokens(max);
        let equal_tokens = option_u64_to_tokens(equal);

        let min_err_param_quoted = if let Some(v) = min {
            quote!(err.add_param(::std::borrow::Cow::from("min"), &#v);)
        } else {
            quote!()
        };
        let max_err_param_quoted = if let Some(v) = max {
            quote!(err.add_param(::std::borrow::Cow::from("max"), &#v);)
        } else {
            quote!()
        };
        let equal_err_param_quoted = if let Some(v) = equal {
            quote!(err.add_param(::std::borrow::Cow::from("equal"), &#v);)
        } else {
            quote!()
        };

        let quoted_error = quote_error(&validation);
        let quoted = quote!(
            if !::validator::validate_length(
                ::validator::Validator::Length {
                    min: #min_tokens,
                    max: #max_tokens,
                    equal: #equal_tokens
                },
                #validator_param
            ) {
                #quoted_error
                #min_err_param_quoted
                #max_err_param_quoted
                #equal_err_param_quoted
                err.add_param(::std::borrow::Cow::from("value"), &#validator_param);
                errors.add(#field_name, err);
            }
        );

        return field_quoter.wrap_if_option(quoted);
    }

    unreachable!()
}

pub fn quote_range_validation(field_quoter: &FieldQuoter, validation: &FieldValidation) -> quote::Tokens {
    let field_name = &field_quoter.name;
    let quoted_ident = field_quoter.quote_validator_param();

    if let Validator::Range { min, max } = validation.validator {
        let quoted_error = quote_error(&validation);
        let min_err_param_quoted = quote!(err.add_param(::std::borrow::Cow::from("min"), &#min););
        let max_err_param_quoted = quote!(err.add_param(::std::borrow::Cow::from("max"), &#max););
        let quoted = quote!(
            if !::validator::validate_range(
                ::validator::Validator::Range {min: #min, max: #max},
                #quoted_ident as f64
            ) {
                #quoted_error
                #min_err_param_quoted
                #max_err_param_quoted
                err.add_param(::std::borrow::Cow::from("value"), &#quoted_ident);
                errors.add(#field_name, err);
            }
        );

        return field_quoter.wrap_if_option(quoted);
    }

    unreachable!()
}

pub fn quote_credit_card_validation(field_quoter: &FieldQuoter, validation: &FieldValidation) -> quote::Tokens {
    let field_name = &field_quoter.name;
    let validator_param = field_quoter.quote_validator_param();

    let quoted_error = quote_error(&validation);
    let quoted = quote!(
        if !::validator::validate_credit_card(#validator_param) {
            #quoted_error
            err.add_param(::std::borrow::Cow::from("value"), &#validator_param);
            errors.add(#field_name, err);
        }
    );

    field_quoter.wrap_if_option(quoted)
}

#[cfg(feature = "phone")]
pub fn quote_phone_validation(field_quoter: &FieldQuoter, validation: &FieldValidation) -> quote::Tokens {
    let field_name = &field_quoter.name;
    let validator_param = field_quoter.quote_validator_param();

    let quoted_error = quote_error(&validation);
    let quoted = quote!(
        if !::validator::validate_phone(#validator_param) {
            #quoted_error
            err.add_param(::std::borrow::Cow::from("value"), &#validator_param);
            errors.add(#field_name, err);
        }
    );

    field_quoter.wrap_if_option(quoted)
}

pub fn quote_url_validation(field_quoter: &FieldQuoter, validation: &FieldValidation) -> quote::Tokens {
    let field_name = &field_quoter.name;
    let validator_param = field_quoter.quote_validator_param();

    let quoted_error = quote_error(&validation);
    let quoted = quote!(
        if !::validator::validate_url(#validator_param) {
            #quoted_error
            err.add_param(::std::borrow::Cow::from("value"), &#validator_param);
            errors.add(#field_name, err);
        }
    );

    field_quoter.wrap_if_option(quoted)
}

pub fn quote_email_validation(field_quoter: &FieldQuoter, validation: &FieldValidation) -> quote::Tokens {
    let field_name = &field_quoter.name;
    let validator_param = field_quoter.quote_validator_param();

    let quoted_error = quote_error(&validation);
    let quoted = quote!(
        if !::validator::validate_email(#validator_param) {
            #quoted_error
            err.add_param(::std::borrow::Cow::from("value"), &#validator_param);
            errors.add(#field_name, err);
        }
    );

    field_quoter.wrap_if_option(quoted)
}

pub fn quote_must_match_validation(field_quoter: &FieldQuoter, validation: &FieldValidation) -> quote::Tokens {
    let ident = &field_quoter.ident;
    let field_name = &field_quoter.name;

    if let Validator::MustMatch(ref other) = validation.validator {
        let other_ident = syn::Ident::from(other.clone());
        let quoted_error = quote_error(&validation);
        let quoted = quote!(
            if !::validator::validate_must_match(&self.#ident, &self.#other_ident) {
                #quoted_error
                err.add_param(::std::borrow::Cow::from("value"), &self.#ident);
                err.add_param(::std::borrow::Cow::from("other"), &self.#other_ident);
                errors.add(#field_name, err);
            }
        );

        return field_quoter.wrap_if_option(quoted);
    }

    unreachable!();
}

pub fn quote_custom_validation(field_quoter: &FieldQuoter, validation: &FieldValidation) -> quote::Tokens {
    let field_name = &field_quoter.name;
    let validator_param = field_quoter.quote_validator_param();

    if let Validator::Custom(ref fun) = validation.validator {
        let fn_ident: syn::Path = syn::parse_str(fun).unwrap();
        let add_message_quoted = if let Some(ref m) = validation.message {
            quote!(err.message = Some(::std::borrow::Cow::from(#m));)
        } else {
            quote!()
        };

        let quoted = quote!(
            match #fn_ident(#validator_param) {
                ::std::result::Result::Ok(()) => (),
                ::std::result::Result::Err(mut err) => {
                    #add_message_quoted
                    err.add_param(::std::borrow::Cow::from("value"), &#validator_param);
                    errors.add(#field_name, err);
                },
            };
        );

        return field_quoter.wrap_if_option(quoted);
    }

    unreachable!();
}

pub fn quote_contains_validation(field_quoter: &FieldQuoter, validation: &FieldValidation) -> quote::Tokens {
    let field_name = &field_quoter.name;
    let validator_param = field_quoter.quote_validator_param();

    if let Validator::Contains(ref needle) = validation.validator {
        let quoted_error = quote_error(&validation);
        let quoted = quote!(
            if !::validator::validate_contains(#validator_param, &#needle) {
                #quoted_error
                err.add_param(::std::borrow::Cow::from("value"), &#validator_param);
                err.add_param(::std::borrow::Cow::from("needle"), &#needle);
                errors.add(#field_name, err);
            }
        );

        return field_quoter.wrap_if_option(quoted);
    }

    unreachable!();
}

pub fn quote_regex_validation(field_quoter: &FieldQuoter, validation: &FieldValidation) -> quote::Tokens {
    let field_name = &field_quoter.name;
    let validator_param = field_quoter.quote_validator_param();

    if let Validator::Regex(ref re) = validation.validator {
        let re_ident: syn::Path = syn::parse_str(re).unwrap();
        let quoted_error = quote_error(&validation);
        let quoted = quote!(
            if !#re_ident.is_match(#validator_param) {
                #quoted_error
                err.add_param(::std::borrow::Cow::from("value"), &#validator_param);
                errors.add(#field_name, err);
            }
        );

        return field_quoter.wrap_if_option(quoted);
    }

    unreachable!();
}

pub fn quote_field_validation(field_quoter: &FieldQuoter, validation: &FieldValidation) -> quote::Tokens {
    match validation.validator {
        Validator::Length {..} => quote_length_validation(&field_quoter, validation),
        Validator::Range {..} => quote_range_validation(&field_quoter, validation),
        Validator::Email => quote_email_validation(&field_quoter, validation),
        Validator::Url => quote_url_validation(&field_quoter, validation),
        Validator::MustMatch(_) => quote_must_match_validation(&field_quoter, validation),
        Validator::Custom(_) => quote_custom_validation(&field_quoter, validation),
        Validator::Contains(_) => quote_contains_validation(&field_quoter, validation),
        Validator::Regex(_) => quote_regex_validation(&field_quoter, validation),
        Validator::CreditCard => quote_credit_card_validation(&field_quoter, validation),
        #[cfg(feature = "phone")]
        Validator::Phone => quote_phone_validation(&field_quoter, validation),
    }
}


pub fn quote_schema_validation(validation: Option<SchemaValidation>) -> quote::Tokens {
    if let Some(v) = validation {
        let fn_ident = syn::Ident::from(v.function);

        let add_message_quoted = if let Some(ref m) = v.message {
            quote!(err.message = Some(::std::borrow::Cow::from(#m));)
        } else {
            quote!()
        };
        let mut_err_token = if v.message.is_some() { quote!(mut) } else { quote!() };
        let quoted = quote!(
            match #fn_ident(self) {
                ::std::result::Result::Ok(()) => (),
                ::std::result::Result::Err(#mut_err_token err) => {
                    #add_message_quoted
                    errors.add("__all__", err);
                },
            };
        );

        if !v.skip_on_field_errors {
            return quoted;
        }

        quote!(
            if errors.is_empty() {
                #quoted
            }
        )
    } else {
        quote!()
    }
}
