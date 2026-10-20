//! Ferramenta Medir: dois cliques com prévia. O primeiro marca o ponto de
//! partida, a prévia segue o mouse e o segundo grava a medida (um passo de Undo).
//! Esc cancela a medida em andamento; `measure.clear` remove as gravadas.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Clique da ferramenta Medir em coordenadas normalizadas da viewport. O ponto
    /// é um vértice próximo (imã), a superfície sob o ponteiro ou o chão.
    pub fn measure_click(&mut self, norm_x: f32, norm_y: f32) -> bool {
        let Some(point) = self.pick_world_point(norm_x, norm_y) else {
            return false;
        };
        let Some(mut pending) = self.state.session.tools.active_measurement.take() else {
            self.state.session.tools.active_measurement = Some(petunia_core::MeasurementItem::new(
                String::new(),
                point.to_array(),
                point.to_array(),
                0.0,
            ));
            self.state
                .set_status("Measure: click the second point (Esc cancels)");
            return true;
        };
        let start = glam::Vec3::from(pending.start);
        pending.end = point.to_array();
        pending.distance = start.distance(point);
        pending.name = format!("Medida {}", self.state.project.measurements.len() + 1);
        let command = petunia_core::AddMeasurementCmd {
            item: pending.clone(),
        };
        match self.state.dispatch(&command) {
            Ok(()) => true,
            Err(error) => {
                // Os dois pontos coincidem (ou algo inválido): mantém o primeiro e
                // espera outro clique, em vez de perder o que já foi marcado.
                self.state.set_status(error.to_string());
                pending.end = pending.start;
                pending.distance = 0.0;
                self.state.session.tools.active_measurement = Some(pending);
                false
            }
        }
    }

    /// Prévia: com um ponto já marcado, a outra ponta segue o ponteiro.
    pub fn measure_hover(&mut self, norm_x: f32, norm_y: f32) -> bool {
        if self.state.session.tools.active_measurement.is_none() {
            return false;
        }
        let Some(point) = self.pick_world_point(norm_x, norm_y) else {
            return false;
        };
        if let Some(pending) = self.state.session.tools.active_measurement.as_mut() {
            pending.end = point.to_array();
            pending.distance = glam::Vec3::from(pending.start).distance(point);
        }
        true
    }

    /// Esc: descarta a medida em andamento. `false` se não havia nenhuma.
    pub fn cancel_measure(&mut self) -> bool {
        if self.state.session.tools.active_measurement.take().is_none() {
            return false;
        }
        self.state.set_status("Measure cancelled.");
        true
    }
}
