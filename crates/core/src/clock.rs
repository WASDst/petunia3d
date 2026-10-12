//! Relógio injetado (website/docs/02-application/commands-events-services.md,
//! "Side effects"): construtores não consultam o relógio do sistema; o host
//! escolhe a fonte de tempo e os testes fixam o instante.

/// Segundos desde a época Unix.
pub trait Clock: Send + Sync + std::fmt::Debug {
    fn now_secs(&self) -> u64;
}

/// Relógio do sistema operacional (padrão do host desktop).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_secs(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs())
    }
}

/// Relógio fixo para testes e execuções determinísticas.
#[derive(Debug, Clone, Copy, Default)]
pub struct FixedClock(pub u64);

impl Clock for FixedClock {
    fn now_secs(&self) -> u64 {
        self.0
    }
}
