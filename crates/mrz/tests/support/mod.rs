use mrz::{DateRole, MrzDate, RawDateField, CURRENT_YY};

pub fn birth(raw: &str) -> MrzDate {
    MrzDate::from_field(
        RawDateField::try_from(raw).expect("six MRZ date characters"),
        DateRole::Birth,
        CURRENT_YY,
    )
}

pub fn expiry(raw: &str) -> MrzDate {
    MrzDate::from_field(
        RawDateField::try_from(raw).expect("six MRZ date characters"),
        DateRole::Expiry,
        CURRENT_YY,
    )
}
