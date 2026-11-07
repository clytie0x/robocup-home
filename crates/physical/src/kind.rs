/// How a number is allowed to be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Measured,
    Configured,
    Derived,
    Assumed,
    SpecifiedScenario,
    ReferenceCode,
    Inferred,
    Unknown,
    UpstreamNominal,
    Blocked,
}

impl Kind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Measured => "measured",
            Kind::Configured => "configured",
            Kind::Derived => "derived",
            Kind::Assumed => "assumed",
            Kind::SpecifiedScenario => "specified-scenario",
            Kind::ReferenceCode => "reference-code",
            Kind::Inferred => "inferred",
            Kind::Unknown => "unknown",
            Kind::UpstreamNominal => "upstream-nominal",
            Kind::Blocked => "blocked",
        }
    }
}
