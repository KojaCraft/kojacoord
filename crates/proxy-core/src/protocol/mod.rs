pub mod coverage;
pub mod dimension_codec;

pub use coverage::{
    ConverterBuilder, ConverterInfo, CoverageStatus, ProtocolCoverage, VersionPair,
};
pub use dimension_codec::{
    build_minimal_dimension_codec, build_minimal_registry, determine_injection_mode,
    needs_codec_injection, uses_dimension_codec, CodecInjectionMode,
};
