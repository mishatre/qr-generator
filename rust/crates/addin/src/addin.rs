//! The object the 1C platform sees.
//!
//! Script name: `AddIn.QrGenerator.QrGenerator`, after
//! `ПодключитьВнешнююКомпоненту(..., "QrGenerator", ТипВнешнейКомпоненты.Native)`.
//! The second `QrGenerator` is the name this object registers with the platform
//! (`ru(QrGenerator)` below), and the first is the connection name the caller
//! picks. `GetClassNames` is not involved in either: it answers with the internal
//! class id `"1"`, which the platform passes back to `GetClassObject`.
//!
//! Three methods are exposed and none of them can raise:
//!
//! | 1C | Returns |
//! |---|---|
//! | `СоздатьQRКод(Данные, Текст, Настройки)` | the PNG as binary data, or an empty value on failure |
//! | `ИдентификаторЭкземпляра()` | eight characters of instance identifier, or empty on failure |
//! | `ПроверочныйКод(Основа)` | four characters of visual/check code |
//! | `ТекстПоследнейОшибки()` | why the last call produced nothing |
//! | `Версия()` | the component version |
//!
//! ## Why a failure is not an error
//!
//! The framework answers the platform with a bare `false` when a method returns
//! `Err`, and 1C turns that into `Ошибка при вызове метода контекста (СоздатьQRКод)`
//! with nothing behind it — no message, no mention of which parameter was wrong.
//! This component is called with a payload a caller cannot fully predict (a code
//! that may be too long, an option that may be misspelled), so a message is the
//! difference between a five-minute fix and an afternoon.
//!
//! So `СоздатьQRКод` answers `Ok` whatever happens, with an empty result on
//! failure, and `ТекстПоследнейОшибки` carries the reason. The BSL wrapper turns
//! that pair into an ordinary 1C exception. The empty value is the sentinel
//! because a real picture is never empty, and because the platform hands an empty
//! binary value over as `ДвоичныеДанные` of size zero, which a script can test
//! without any help from the component.

use crate::code;
use crate::error::QrError;
use crate::render;
use ones_addin::ffi::{Connection, ParamValue};
use ones_addin_derive::{component, function, methods_impl, AddInComponent};

/// The 1C-facing add-in object.
///
/// `data` and `connection` are the framework's fields, not this component's:
/// `#[derive(AddInComponent)]` writes into both by name when the platform
/// initialises the object, which is why they exist here at all. `data` is not read
/// by anything — the description of the object it carries is of no use here.
///
/// `last_error` is state, and that is a deliberate departure from the sibling
/// project, where the wrapper keeps nothing. A component object lives as long as
/// the session that connected it, so remembering one string is safe; the
/// alternative is a second method call that re-renders the picture to find out
/// what went wrong with the first.
#[allow(dead_code)]
#[derive(AddInComponent)]
#[component(ru(QrGenerator))]
pub struct QrGenerator {
    data: String,
    connection: Option<&'static Connection>,
    last_error: String,
}

// `new` exists because the framework's `GetClassObject` calls it, not because the
// object has a meaningful default: a `Default` implementation would be a second way
// to build the one object this library carries, with no caller.
#[allow(clippy::new_without_default)]
impl QrGenerator {
    /// Construct the object.
    ///
    /// `export_addin!` in `lib.rs` generates `GetClassObject` as a call to
    /// `QrGenerator::new()`, so this is part of the ABI contract with the framework
    /// rather than a convenience.
    pub fn new() -> Self {
        Self {
            data: String::new(),
            connection: None,
            last_error: String::new(),
        }
    }

    /// Why the last call to `generate` produced nothing.
    ///
    /// The 1C-facing accessor is `ТекстПоследнейОшибки`, generated from the method
    /// below; this one is what the Rust tests read.
    pub fn error_message(&self) -> &str {
        &self.last_error
    }
}

/// The methods 1C can call.
///
/// The `&Connection` the framework always passes comes first and is unused here;
/// the parameters the script supplies follow it as `&ParamValue`. The first
/// parameter after those is the `blob` return, which tells `#[methods_impl]` to
/// hand the platform binary data instead of a string — the marker the vendored
/// macro gained for this component, recorded in `rust/vendor/README.md`.
#[methods_impl]
impl QrGenerator {
    /// Render the code.
    ///
    /// `Данные` is what the code carries, `Текст` is the label drawn on the plate in
    /// the middle of it (empty for no label at all), and `НастройкиJSON` is an
    /// optional JSON object of options — see `docs/plan/qr-contract.md` for every
    /// key. The result is a PNG, or empty on failure.
    #[function(ru("СоздатьQRКод"), en("Generate"), blob)]
    fn generate(
        &mut self,
        _: &Connection,
        data: &ParamValue,
        text: &ParamValue,
        options_json: &ParamValue,
    ) -> Result<Vec<u8>, QrError> {
        let outcome = read_arguments(data, text, options_json)
            .and_then(|(data, text, options_json)| render::render(&data, &text, &options_json));

        match outcome {
            Ok(png) => {
                // Clear the previous failure, so a caller that checks the message
                // after a successful call is not told about the call before it.
                self.last_error.clear();

                Ok(png)
            }
            Err(error) => {
                self.last_error = error.to_string();

                Ok(Vec::new())
            }
        }
    }

    /// The message from the last failed `СоздатьQRКод`, or an empty string.
    #[function(ru("ТекстПоследнейОшибки"), en("LastError"))]
    fn last_error(&self, _: &Connection) -> Result<String, QrError> {
        Ok(self.last_error.clone())
    }

    /// A fresh instance identifier: eight characters, five of them from random.
    ///
    /// The consumer's records carry one in the last field, and 1C has no random
    /// source this format was built on. An empty string means the operating system
    /// refused to supply random bytes, which `ТекстПоследнейОшибки` explains.
    #[function(ru("ИдентификаторЭкземпляра"), en("InstanceId"))]
    fn instance_id(&mut self, _: &Connection) -> Result<String, QrError> {
        match code::instance_id() {
            Ok(identifier) => {
                self.last_error.clear();

                Ok(identifier)
            }
            Err(error) => {
                self.last_error = error.to_string();

                Ok(String::new())
            }
        }
    }

    /// The four character check code for the stem of a record.
    ///
    /// `Основа` is the record without its code — everything up to and including the
    /// eight character identifier. See `docs/plan/qr-contract.md`: what is hashed is
    /// the one thing about this format that is easy to get wrong, and it is why this
    /// is a function of the component rather than a line of BSL.
    #[function(ru("ПроверочныйКод"), en("CheckCode"))]
    fn check_code(&self, _: &Connection, stem: &ParamValue) -> Result<String, QrError> {
        Ok(code::check_code(&argument_as_str("Основа", stem)?))
    }

    /// The component version, for logging and support requests.
    #[function(ru("Версия"), en("Version"))]
    fn version(&self, _: &Connection) -> Result<String, QrError> {
        Ok(version())
    }
}

/// The version this library was built as.
pub fn version() -> String {
    String::from(env!("CARGO_PKG_VERSION"))
}

/// Read the three parameters the script passed.
///
/// An empty `Данные` is refused here rather than passed to the encoder: an empty
/// QR code is a valid symbol that scans to nothing, which looks like success and is
/// never what the caller meant.
fn read_arguments(
    data: &ParamValue,
    text: &ParamValue,
    options_json: &ParamValue,
) -> Result<(String, String, String), QrError> {
    let data = argument_as_str("Данные", data)?;

    if data.trim().is_empty() {
        return Err(QrError::argument(
            "the Данные parameter must not be empty: the code would carry nothing",
        ));
    }

    Ok((
        data,
        argument_as_str("Текст", text)?,
        argument_as_str("НастройкиJSON", options_json)?,
    ))
}

/// Read one script-supplied parameter as a string.
///
/// The framework hands a method a `tVariant` and coerces nothing, so this is where
/// the ABI's "every value is a variant" meets the typed world. `Empty` is the
/// absent optional parameter and means the empty string, which is how the BSL
/// wrapper says "no label" and "no options".
///
/// Any other type is a mistake in the calling script, and it is reported rather
/// than raised: the platform's error path carries no message, so a component that
/// raises on a wrong argument type tells the caller less than one that says what it
/// received.
fn argument_as_str(name: &str, value: &ParamValue) -> Result<String, QrError> {
    match value {
        ParamValue::Str(units) => {
            // The ABI hands the terminating null over as part of the slice. It must
            // not reach the encoder, where it would become a byte of the payload.
            let text = match units.split_last() {
                Some((&0, rest)) => rest,
                _ => units,
            };

            Ok(String::from_utf16_lossy(text))
        }
        ParamValue::Empty => Ok(String::new()),
        other => Err(QrError::argument(format!(
            "the {name} parameter must be a string, but the call passed {}",
            describe(other)
        ))),
    }
}

/// Name a parameter type the way the 1C developer who passed it will recognise it.
fn describe(value: &ParamValue) -> &'static str {
    match value {
        ParamValue::Empty => "an empty value",
        ParamValue::Bool(_) => "a boolean",
        ParamValue::I32(_) | ParamValue::F64(_) => "a number",
        ParamValue::Date(_) => "a date",
        ParamValue::Str(_) => "a string",
        ParamValue::Blob(_) => "binary data",
    }
}

#[cfg(test)]
mod tests {
    use super::{argument_as_str, describe, read_arguments, version, ParamValue, QrGenerator};
    use crate::error::ErrorKind;

    /// The ABI hands the terminating null over as part of the slice. It must not
    /// reach the encoder, where it would become a byte of the QR payload.
    #[test]
    fn a_string_parameter_loses_its_terminating_null() {
        let units: Vec<u16> = "заказ\0".encode_utf16().collect();

        assert_eq!(
            argument_as_str("Данные", &ParamValue::Str(&units)).expect("a string parameter"),
            "заказ"
        );
    }

    /// An absent optional parameter arrives as `Empty`, and that is how the BSL
    /// wrapper says "no label".
    #[test]
    fn an_absent_parameter_is_the_empty_string() {
        assert_eq!(
            argument_as_str("Текст", &ParamValue::Empty).expect("an absent parameter"),
            ""
        );
    }

    #[test]
    fn a_parameter_of_another_type_is_reported_rather_than_raised() {
        let error =
            argument_as_str("Данные", &ParamValue::I32(42)).expect_err("a number is not a string");

        assert_eq!(error.kind, ErrorKind::Argument);
        assert!(
            error.message.contains("a number"),
            "the message must name what the call passed, but reads: {}",
            error.message
        );
    }

    #[test]
    fn the_first_wrong_parameter_is_the_one_reported() {
        let error = read_arguments(
            &ParamValue::I32(42),
            &ParamValue::Bool(true),
            &ParamValue::Empty,
        )
        .expect_err("neither parameter is a string");

        assert!(
            error.message.contains("Данные"),
            "the message must name the parameter, but reads: {}",
            error.message
        );
    }

    #[test]
    fn an_empty_payload_is_refused_before_the_encoder_sees_it() {
        let empty: Vec<u16> = "\0".encode_utf16().collect();

        let error = read_arguments(
            &ParamValue::Str(&empty),
            &ParamValue::Empty,
            &ParamValue::Empty,
        )
        .expect_err("an empty payload is not a code");

        assert_eq!(error.kind, ErrorKind::Argument);
        assert!(error.message.contains("Данные"), "reads: {}", error.message);
    }

    #[test]
    fn every_parameter_type_has_a_name() {
        // `describe` is what turns a wrong-type failure into a sentence a 1C
        // developer can act on, so no variant may be left unnamed.
        assert_eq!(describe(&ParamValue::Empty), "an empty value");
        assert_eq!(describe(&ParamValue::Bool(true)), "a boolean");
        assert_eq!(describe(&ParamValue::I32(1)), "a number");
        assert_eq!(describe(&ParamValue::F64(1.0)), "a number");
        assert_eq!(describe(&ParamValue::Str(&[])), "a string");
        assert_eq!(describe(&ParamValue::Blob(&[])), "binary data");
    }

    #[test]
    fn the_object_starts_with_no_failure_to_report() {
        // The wrapper reads the message after a call that produced nothing; an
        // object that started out holding one would blame a call that never
        // happened.
        assert!(QrGenerator::new().error_message().is_empty());
    }

    #[test]
    fn the_version_is_the_one_the_manifest_declares() {
        assert_eq!(version(), "0.1.0");
    }
}
