use std::collections::HashMap;

use crate::{Field, ValidationEnv, ValidationError, ValidationErrors, Value};

/// A value that can be stored in a [`FormControl`] and converted to and from
/// the [`Value`] type used by validators.
///
/// Implemented for `String`, `bool`, the numeric primitives, and `Option<T>`
/// where `T` is a control value.
pub trait ControlValue: Sized + Send + Sync + 'static {
    /// Converts a validator [`Value`] into this type.
    ///
    /// # Errors
    ///
    /// Returns an error when the value is not compatible with this type, for
    /// example a non-numeric value for a number control.
    fn from_value(value: Value) -> Result<Self, ValidationError>;

    /// Converts this type into a validator [`Value`].
    fn to_value(&self) -> Value;

    /// Renders this value as a string suitable for HTML form attributes such as
    /// `value` or `data-control-value`.
    fn to_control_string(&self) -> String;
}

impl ControlValue for String {
    fn from_value(value: Value) -> Result<Self, ValidationError> {
        match value {
            Value::Missing => Ok(String::new()),
            Value::String(text) => Ok(text),
            _ => Err("expected a string".into()),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.clone())
    }

    fn to_control_string(&self) -> String {
        self.clone()
    }
}

impl ControlValue for bool {
    fn from_value(value: Value) -> Result<Self, ValidationError> {
        match value {
            Value::Missing => Ok(false),
            Value::Bool(value) => Ok(value),
            _ => Err("expected a boolean".into()),
        }
    }

    fn to_value(&self) -> Value {
        Value::Bool(*self)
    }

    fn to_control_string(&self) -> String {
        if *self {
            "true".to_string()
        } else {
            "false".to_string()
        }
    }
}

macro_rules! impl_control_value_numeric {
    ($($ty:ty),* $(,)?) => {
        $(
            // The validator pipeline normalizes numbers to `f64`; converting
            // to and from the target primitive is inherently a narrowing or
            // widening cast.
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_precision_loss,
                clippy::cast_lossless,
                clippy::cast_sign_loss
            )]
            impl ControlValue for $ty {
                fn from_value(value: Value) -> Result<Self, ValidationError> {
                    match value {
                        Value::Missing => Ok(0 as $ty),
                        Value::Number(number) => Ok(number as $ty),
                        _ => Err("expected a number".into()),
                    }
                }

                fn to_value(&self) -> Value {
                    Value::Number(*self as f64)
                }

                fn to_control_string(&self) -> String {
                    self.to_string()
                }
            }
        )*
    };
}

impl_control_value_numeric!(
    f64, f32, i128, i64, i32, i16, i8, u128, u64, u32, u16, u8, isize, usize,
);

impl<T: ControlValue> ControlValue for Option<T> {
    fn from_value(value: Value) -> Result<Self, ValidationError> {
        match value {
            Value::Missing => Ok(None),
            value => T::from_value(value).map(Some),
        }
    }

    fn to_value(&self) -> Value {
        match self {
            Some(value) => value.to_value(),
            None => Value::Missing,
        }
    }

    fn to_control_string(&self) -> String {
        match self {
            Some(value) => value.to_control_string(),
            None => String::new(),
        }
    }
}

/// A typed form control that holds a single value and its validator chain.
///
/// Controls are created by a [`FormGroup`] and can be validated individually or
/// as part of the group.
pub struct FormControl<T: ControlValue> {
    value: T,
    field: Field,
    errors: Vec<ValidationError>,
    dirty: bool,
    touched: bool,
}

impl<T: ControlValue> FormControl<T> {
    /// Creates a control with an initial value and a validator chain.
    #[must_use]
    pub fn new(value: T, field: Field) -> Self {
        Self {
            value,
            field,
            errors: Vec::new(),
            dirty: false,
            touched: false,
        }
    }

    /// Returns the current value.
    #[must_use]
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Sets the current value directly, without running validators.
    pub fn set_value(&mut self, value: T) {
        self.value = value;
    }

    /// Sets the value from a raw validator [`Value`], running the validator
    /// chain in the given environment and recording any errors.
    ///
    /// On success the control value is updated to the parsed value. On failure
    /// the control keeps the raw value so it can be rendered back to the user.
    ///
    /// # Errors
    ///
    /// Returns the first validation error for this control.
    pub fn set_raw_value(
        &mut self,
        env: ValidationEnv,
        raw_value: Value,
    ) -> Result<(), ValidationError> {
        let mut value = raw_value;
        self.errors.clear();

        for entry in self.field.validators() {
            if !entry.env.includes(env) {
                continue;
            }

            match entry.validator.validate(&value, env) {
                Ok(new_value) => value = new_value,
                Err(error) => {
                    self.errors.push(error.clone());
                    if let Ok(typed) = T::from_value(value) {
                        self.value = typed;
                    }
                    return Err(error);
                }
            }
        }

        match T::from_value(value) {
            Ok(typed) => {
                self.value = typed;
                Ok(())
            }
            Err(error) => {
                self.errors.push(error.clone());
                Err(error)
            }
        }
    }

    /// Validates the current value in the given environment.
    ///
    /// # Errors
    ///
    /// Returns the first validation error for this control.
    pub fn validate(&mut self, env: ValidationEnv) -> Result<(), ValidationError> {
        self.set_raw_value(env, self.value.to_value())
    }

    /// Returns the recorded errors for this control.
    #[must_use]
    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }

    /// Returns `true` if the control has no recorded errors.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns `true` if the control has recorded errors.
    #[must_use]
    pub fn invalid(&self) -> bool {
        !self.valid()
    }

    /// Returns `true` if the value has been changed from its initial state.
    #[must_use]
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Marks the control as changed.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Returns `true` if the control has lost focus.
    #[must_use]
    pub fn touched(&self) -> bool {
        self.touched
    }

    /// Marks the control as having lost focus.
    pub fn mark_touched(&mut self) {
        self.touched = true;
    }
}

/// A type-erased control so controls of different types can live in the same
/// map.
pub trait AnyControl: Send + Sync {
    /// Returns the type-erased underlying `Any` reference.
    fn as_any(&self) -> &dyn std::any::Any;
    /// Returns the mutable type-erased underlying `Any` reference.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    /// Returns the recorded errors for this control.
    fn errors(&self) -> &[ValidationError];
    /// Returns `true` if the control has no recorded errors.
    fn valid(&self) -> bool;
    /// Returns `true` if the control has recorded errors.
    fn invalid(&self) -> bool;
    /// Returns `true` if the value has been changed from its initial state.
    fn dirty(&self) -> bool;
    /// Returns `true` if the control has lost focus.
    fn touched(&self) -> bool;
    /// Sets the raw value from a validator [`Value`] in the given environment.
    ///
    /// # Errors
    ///
    /// Returns the first validation error for this control.
    fn set_raw_value(&mut self, env: ValidationEnv, value: Value) -> Result<(), ValidationError>;
    /// Returns the current value as a validator [`Value`].
    fn to_value(&self) -> Value;
}

impl<T: ControlValue> AnyControl for FormControl<T> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn errors(&self) -> &[ValidationError] {
        self.errors()
    }

    fn valid(&self) -> bool {
        self.valid()
    }

    fn invalid(&self) -> bool {
        self.invalid()
    }

    fn dirty(&self) -> bool {
        self.dirty()
    }

    fn touched(&self) -> bool {
        self.touched()
    }

    fn set_raw_value(&mut self, env: ValidationEnv, value: Value) -> Result<(), ValidationError> {
        self.set_raw_value(env, value)
    }

    fn to_value(&self) -> Value {
        self.value().to_value()
    }
}

/// A collection of named [`FormControl`]s that can be validated together and
/// assembled into a target struct `T`.
///
/// `T` is usually a form struct that derives [`ValidForm`](crate::ValidForm),
/// which generates the [`FormGroupSchema`] implementation that builds a group
/// from the struct's fields and assembles it back into `T`.
/// Assembles a group's controls into the target struct `T`.
type Assembler<T> = Box<dyn Fn(&FormGroup<T>) -> Result<T, ValidationErrors> + Send + Sync>;

pub struct FormGroup<T> {
    controls: HashMap<String, Box<dyn AnyControl>>,
    assembler: Option<Assembler<T>>,
}

impl<T> FormGroup<T> {
    /// Creates an empty group. The derive-generated implementation fills the
    /// controls map and installs the assembler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            controls: HashMap::new(),
            assembler: None,
        }
    }

    /// Adds a named control to the group.
    pub fn control<C: ControlValue>(&mut self, name: impl Into<String>, control: FormControl<C>) {
        self.controls.insert(name.into(), Box::new(control));
    }

    /// Installs the function that assembles the group's controls into `T`.
    pub fn assembler(
        &mut self,
        assembler: impl Fn(&FormGroup<T>) -> Result<T, ValidationErrors> + Send + Sync + 'static,
    ) {
        self.assembler = Some(Box::new(assembler));
    }

    /// Returns a reference to the named control, if it exists and has the
    /// requested type.
    #[must_use]
    pub fn get<C: ControlValue>(&self, name: &str) -> Option<&FormControl<C>> {
        self.controls
            .get(name)
            .and_then(|any| any.as_any().downcast_ref::<FormControl<C>>())
    }

    /// Returns a mutable reference to the named control, if it exists and has
    /// the requested type.
    #[must_use]
    pub fn get_mut<C: ControlValue>(&mut self, name: &str) -> Option<&mut FormControl<C>> {
        self.controls
            .get_mut(name)
            .and_then(|any| any.as_any_mut().downcast_mut::<FormControl<C>>())
    }

    /// Iterates over the group's controls as `(name, control)` pairs.
    pub fn controls(&self) -> impl Iterator<Item = (&str, &dyn AnyControl)> {
        self.controls
            .iter()
            .map(|(name, control)| (name.as_str(), &**control))
    }

    /// Validates every control and assembles the group into `T`.
    ///
    /// The assembler is installed by the [`ValidForm`](crate::ValidForm)
    /// derive. Calling this on a group without an assembler, such as one built
    /// with [`FormGroup::new`], returns an error.
    ///
    /// # Errors
    ///
    /// Returns a [`ValidationErrors`] collection when any control fails
    /// validation or the group has no assembler.
    pub fn value(&self) -> Result<T, ValidationErrors> {
        let Some(assembler) = &self.assembler else {
            let mut errors = ValidationErrors::new();
            errors.push("", "form group has no assembler");
            return Err(errors);
        };
        assembler(self)
    }

    /// Returns all recorded errors across every control.
    #[must_use]
    pub fn errors(&self) -> ValidationErrors {
        let mut errors = ValidationErrors::new();
        for (name, control) in &self.controls {
            for error in control.errors() {
                errors.push(name.clone(), error.clone());
            }
        }
        errors
    }

    /// Returns `true` if every control is valid.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.controls.values().all(|control| control.valid())
    }

    /// Returns `true` if any control is invalid.
    #[must_use]
    pub fn invalid(&self) -> bool {
        !self.valid()
    }
}

impl<T> Default for FormGroup<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A type that can be built into a [`FormGroup`] of itself.
///
/// This trait is implemented by the [`ValidForm`](crate::ValidForm) derive
/// macro. Use [`FormGroupSchema::form_group`] for a blank form, or
/// [`FormGroupSchema::form_group_with`] to pre-fill values from an existing
/// value.
pub trait FormGroupSchema: Sized + Send + Sync + 'static {
    /// Builds a form group from the default value of the form type.
    ///
    /// The form type must implement [`Default`].
    fn form_group() -> FormGroup<Self>
    where
        Self: Default;

    /// Builds a form group pre-filled with the values from `default`.
    fn form_group_with(default: &Self) -> FormGroup<Self>;

    /// Builds a form group from the supplied raw values and validates it in the
    /// given environment.
    ///
    /// Missing values are represented by [`Value::Missing`].
    ///
    /// # Errors
    ///
    /// Returns a [`ValidationErrors`] collection when any control fails
    /// validation.
    fn form_group_from_values(
        env: ValidationEnv,
        values: &dyn crate::ValidationData,
    ) -> Result<FormGroup<Self>, ValidationErrors>;

    /// Builds a form group from the supplied raw values, validating each
    /// control but keeping the group even when some controls are invalid.
    ///
    /// Use this to re-render a form after a failed submit so the user's input
    /// and error messages are preserved.
    fn form_group_from_values_lossy(
        env: ValidationEnv,
        values: &dyn crate::ValidationData,
    ) -> FormGroup<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[derive(Debug, Default, PartialEq)]
    struct Sample {
        email: String,
        age: i32,
        referral: Option<String>,
    }

    fn sample_group(default: &Sample) -> FormGroup<Sample> {
        let mut group = FormGroup::new();
        group.control(
            "email",
            FormControl::new(
                default.email.clone(),
                Field::new().string().required().email(),
            ),
        );
        group.control(
            "age",
            FormControl::new(default.age, Field::new().number().required().min(18.0)),
        );
        group.control(
            "referral",
            FormControl::new(default.referral.clone(), Field::new().string()),
        );
        group.assembler(|group| {
            let mut errors = ValidationErrors::new();
            let mut email = None;
            let mut age = None;
            let mut referral = None;

            match group.get::<String>("email") {
                Some(control) if control.valid() => email = Some(control.value().clone()),
                Some(control) => {
                    for error in control.errors() {
                        errors.push("email", error.clone());
                    }
                }
                None => errors.push("email", "control is missing"),
            }
            match group.get::<i32>("age") {
                Some(control) if control.valid() => age = Some(*control.value()),
                Some(control) => {
                    for error in control.errors() {
                        errors.push("age", error.clone());
                    }
                }
                None => errors.push("age", "control is missing"),
            }
            match group.get::<Option<String>>("referral") {
                Some(control) if control.valid() => referral = Some(control.value().clone()),
                Some(control) => {
                    for error in control.errors() {
                        errors.push("referral", error.clone());
                    }
                }
                None => errors.push("referral", "control is missing"),
            }

            if errors.is_empty() {
                Ok(Sample {
                    email: email.unwrap(),
                    age: age.unwrap(),
                    referral: referral.unwrap(),
                })
            } else {
                Err(errors)
            }
        });
        group
    }

    #[test]
    fn control_stores_and_validates_value() {
        let mut control = FormControl::new(String::new(), Field::new().string().required().email());

        assert!(control.value().is_empty());
        assert!(control.valid());

        control.set_value("hello".into());
        assert!(control.validate(ValidationEnv::Server).is_err());
        assert!(control.invalid());

        control.set_value("hello@example.com".into());
        assert!(control.validate(ValidationEnv::Server).is_ok());
        assert!(control.valid());
        assert_eq!(control.value(), "hello@example.com");
    }

    #[test]
    fn required_rejects_missing_and_empty() {
        let mut control = FormControl::new(String::new(), Field::new().string().required());

        assert!(control
            .set_raw_value(ValidationEnv::Server, Value::Missing)
            .is_err());
        assert!(control.invalid());

        assert!(control
            .set_raw_value(ValidationEnv::Server, Value::String(String::new()))
            .is_err());
        assert!(control.invalid());

        assert!(control
            .set_raw_value(ValidationEnv::Server, Value::String("hello".into()))
            .is_ok());
        assert!(control.valid());
    }

    #[test]
    fn number_control_parses_string_input() {
        let mut control = FormControl::new(0_i32, Field::new().number().required().min(18.0));

        assert!(control
            .set_raw_value(ValidationEnv::Server, Value::String("21".into()))
            .is_ok());
        assert_eq!(control.value(), &21_i32);

        assert!(control
            .set_raw_value(ValidationEnv::Server, Value::String("16".into()))
            .is_err());
        assert!(control.invalid());
        // The invalid raw value is kept so it can be rendered back.
        assert_eq!(control.value(), &16_i32);
    }

    #[test]
    fn invalid_raw_value_is_kept_for_rendering() {
        let mut control = FormControl::new(String::new(), Field::new().string().required().email());

        control
            .set_raw_value(ValidationEnv::Server, Value::String("bad".into()))
            .ok();
        assert!(control.invalid());
        assert_eq!(control.value(), "bad");
    }

    #[test]
    fn optional_control_accepts_missing() {
        let mut control: FormControl<Option<String>> =
            FormControl::new(None, Field::new().string().email());

        assert!(control
            .set_raw_value(ValidationEnv::Server, Value::Missing)
            .is_ok());
        assert_eq!(control.value(), &None);
        assert!(control.valid());

        assert!(control
            .set_raw_value(ValidationEnv::Server, Value::String("not-an-email".into()))
            .is_err());
        assert!(control.invalid());
    }

    #[test]
    fn bool_control_parses_common_strings() {
        let mut control = FormControl::new(false, Field::new().bool());

        for value in ["true", "1", "yes", "on"] {
            assert!(control
                .set_raw_value(ValidationEnv::Server, Value::String(value.into()))
                .is_ok());
            assert!(control.value());
        }

        for value in ["false", "0", "no", "off"] {
            assert!(control
                .set_raw_value(ValidationEnv::Server, Value::String(value.into()))
                .is_ok());
            assert!(!control.value());
        }
    }

    #[test]
    fn dirty_and_touched_flags() {
        let mut control = FormControl::new(String::new(), Field::new().string());

        assert!(!control.dirty());
        assert!(!control.touched());

        control.mark_dirty();
        control.mark_touched();

        assert!(control.dirty());
        assert!(control.touched());
    }

    #[test]
    fn group_collects_errors_from_controls() {
        let mut group = sample_group(&Sample::default());

        group
            .get_mut::<String>("email")
            .unwrap()
            .set_value("bad".into());
        group.get_mut::<i32>("age").unwrap().set_value(16);

        group
            .get_mut::<String>("email")
            .unwrap()
            .validate(ValidationEnv::Server)
            .ok();
        group
            .get_mut::<i32>("age")
            .unwrap()
            .validate(ValidationEnv::Server)
            .ok();

        assert!(group.invalid());
        assert!(group.errors().get("email").is_some());
        assert!(group.errors().get("age").is_some());
    }

    #[test]
    fn group_value_assembles_valid_controls() {
        let mut group = sample_group(&Sample::default());

        group
            .get_mut::<String>("email")
            .unwrap()
            .set_raw_value(
                ValidationEnv::Server,
                Value::String("hello@example.com".into()),
            )
            .ok();
        group
            .get_mut::<i32>("age")
            .unwrap()
            .set_raw_value(ValidationEnv::Server, Value::String("21".into()))
            .ok();

        let value = group.value().unwrap();
        assert_eq!(value.email, "hello@example.com");
        assert_eq!(value.age, 21);
        assert_eq!(value.referral, None);
    }

    #[test]
    fn group_value_rejects_invalid_controls() {
        let mut group = sample_group(&Sample::default());

        group
            .get_mut::<String>("email")
            .unwrap()
            .set_raw_value(ValidationEnv::Server, Value::String("bad".into()))
            .ok();
        group
            .get_mut::<i32>("age")
            .unwrap()
            .set_raw_value(ValidationEnv::Server, Value::String("16".into()))
            .ok();

        let errors = group.value().unwrap_err();
        assert!(errors.get("email").is_some());
        assert!(errors.get("age").is_some());
    }

    #[test]
    fn group_without_assembler_cannot_produce_value() {
        let group: FormGroup<Sample> = FormGroup::new();
        assert!(group.value().is_err());
    }

    #[test]
    fn custom_validator_still_works() {
        fn six_chars(value: &Value, _env: ValidationEnv) -> Result<Value, ValidationError> {
            match value {
                Value::Missing => Ok(Value::Missing),
                Value::String(text) if text.len() == 6 => Ok(value.clone()),
                _ => Err("must be 6 characters".into()),
            }
        }

        let mut control = FormControl::new(
            None::<String>,
            Field::new().string().server_only().custom(six_chars),
        );

        // The server-only validator is skipped on the client.
        assert!(control
            .set_raw_value(ValidationEnv::Client, Value::String("bad".into()))
            .is_ok());
        assert!(control
            .set_raw_value(ValidationEnv::Server, Value::String("bad".into()))
            .is_err());
        assert!(control
            .set_raw_value(ValidationEnv::Server, Value::String("abcdef".into()))
            .is_ok());
        assert_eq!(control.value(), &Some("abcdef".into()));
    }
}
