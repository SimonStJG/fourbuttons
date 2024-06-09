use chrono::NaiveDateTime;

#[allow(clippy::struct_field_names)]
#[derive(Debug, PartialEq, Eq, PartialOrd)]
pub(crate) struct ApplicationState {
    pub(crate) take_pills_pending: Option<NaiveDateTime>,
    pub(crate) water_plants_pending: Option<NaiveDateTime>,
    pub(crate) i_pending: Option<NaiveDateTime>,
}
