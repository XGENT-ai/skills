pub mod architecture;
pub mod dataflow;
pub mod lifecycle;
pub mod sequence;
pub mod workflow;

use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::input::{CompileOptions, DiagramInput, DiagramType};
use crate::scene::CompiledDiagram;

pub fn compile(
    input: &DiagramInput,
    options: &CompileOptions,
) -> Result<CompiledDiagram, Diagnostics> {
    if options
        .input_contract
        .is_some_and(|contract| contract != input.input_contract)
        || options
            .quality
            .is_some_and(|quality| quality != input.quality)
        || options.viewer_revision != input.viewer_revision
    {
        return Err(vec![Diagnostic::error(
            Some(input.diagram_type.as_str()),
            "input/config-binding",
            "/meta",
            "Compile options differ from the validated input binding",
        )]);
    }
    match input.diagram_type {
        DiagramType::Architecture => architecture::compile(input, options),
        DiagramType::Workflow => workflow::compile(input, options),
        DiagramType::Sequence => sequence::compile(input, options),
        DiagramType::Dataflow => dataflow::compile(input, options),
        DiagramType::Lifecycle => lifecycle::compile(input, options),
    }
}
