//! Authored reference edits use the same transactional writer as geometry.
use crate::{Command, CommandContext, CommandError};
use petunia_project::{ProjectChanges, RefAxis, ReferenceImage};

#[derive(Debug, Clone)]
pub enum ReferenceOp {
    SetSlot(ReferenceImage),
    Append(ReferenceImage),
    Remove,
    Visible(bool),
    Locked(bool),
    Parameter { field: String, value: f32 },
    Clear,
}
#[derive(Debug, Clone)]
pub struct EditReferenceCmd {
    pub axis: RefAxis,
    /// Pixel/parameter revision observed by the controller. None means a new slot.
    pub expected_revision: Option<u64>,
    pub op: ReferenceOp,
}
fn axis_matches(a: RefAxis, b: RefAxis) -> bool {
    a == b
        || (a == RefAxis::Side && b == RefAxis::Right)
        || (a == RefAxis::Right && b == RefAxis::Side)
}
impl Command for EditReferenceCmd {
    fn label(&self) -> &'static str {
        "edit reference"
    }
    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }
    fn execute(&self, ctx: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let refs = ctx.document.references_mut();
        if matches!(self.op, ReferenceOp::Clear) {
            if refs.is_empty() {
                return Err(CommandError::NoChange("No references".into()));
            }
            if refs.iter().any(|r| r.locked) {
                return Err(CommandError::Execution("Reference is locked".into()));
            }
            refs.clear();
            return Ok(());
        }
        let index = refs.iter().position(|r| axis_matches(r.axis, self.axis));
        if let Some(expected) = self.expected_revision
            && index.is_none_or(|i| refs[i].revision != expected)
        {
            return Err(CommandError::StaleReference("Reference changed".into()));
        }
        match &self.op {
            ReferenceOp::SetSlot(image) | ReferenceOp::Append(image) => {
                let bytes = (image.width as usize)
                    .checked_mul(image.height as usize)
                    .and_then(|n| n.checked_mul(4));
                if image.width == 0
                    || image.height == 0
                    || image.width > 4096
                    || image.height > 4096
                    || bytes != Some(image.rgba.len())
                {
                    return Err(CommandError::Execution(
                        "Invalid reference pixels (maximum 4096 x 4096)".into(),
                    ));
                }
                let mut image = image.clone();
                image.axis = self.axis;
                image.bump_revision();
                if matches!(self.op, ReferenceOp::SetSlot(_)) {
                    if let Some(i) = index {
                        if refs[i].locked {
                            return Err(CommandError::Execution("Reference is locked".into()));
                        }
                        if image.revision == refs[i].revision {
                            image.bump_revision();
                        }
                        image.offset = refs[i].offset;
                        image.size = refs[i].size;
                        image.opacity = refs[i].opacity;
                        image.rotation = refs[i].rotation;
                        image.visible = refs[i].visible;
                        refs[i] = image;
                    } else {
                        refs.push(image)
                    }
                } else {
                    refs.push(image)
                }
            }
            ReferenceOp::Remove => {
                let i = index
                    .ok_or_else(|| CommandError::StaleReference("Reference missing".into()))?;
                if refs[i].locked {
                    return Err(CommandError::Execution("Reference is locked".into()));
                }
                refs.remove(i);
            }
            ReferenceOp::Visible(v) => {
                let i = index
                    .ok_or_else(|| CommandError::StaleReference("Reference missing".into()))?;
                if refs[i].visible == *v {
                    return Err(CommandError::NoChange("Visibility unchanged".into()));
                }
                refs[i].visible = *v;
                refs[i].bump_revision();
            }
            ReferenceOp::Locked(v) => {
                let i = index
                    .ok_or_else(|| CommandError::StaleReference("Reference missing".into()))?;
                if refs[i].locked == *v {
                    return Err(CommandError::NoChange("Lock unchanged".into()));
                }
                refs[i].locked = *v;
                refs[i].bump_revision();
            }
            ReferenceOp::Parameter { field, value } => {
                let i = index
                    .ok_or_else(|| CommandError::StaleReference("Reference missing".into()))?;
                if refs[i].locked {
                    return Err(CommandError::Execution("Reference is locked".into()));
                }
                if !value.is_finite() {
                    return Err(CommandError::Execution(
                        "Non-finite reference parameter".into(),
                    ));
                }
                let r = &mut refs[i];
                let next = match field.as_str() {
                    "opacity" => value.clamp(0., 1.),
                    "size" => value.clamp(0.1, 50.),
                    "offset" => value.clamp(-50., 50.),
                    "rotation" => value.clamp(-180., 180.),
                    _ => {
                        return Err(CommandError::Execution(
                            "Unknown reference parameter".into(),
                        ));
                    }
                };
                let target = match field.as_str() {
                    "opacity" => &mut r.opacity,
                    "size" => &mut r.size,
                    "offset" => &mut r.offset,
                    _ => &mut r.rotation,
                };
                if (*target - next).abs() < f32::EPSILON {
                    return Err(CommandError::NoChange("Parameter unchanged".into()));
                }
                *target = next;
                r.bump_revision();
            }
            ReferenceOp::Clear => {}
        }
        Ok(())
    }
}
