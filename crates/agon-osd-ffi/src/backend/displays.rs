use crate::error::ApiError;
use agon_osd::{DdcTransport, DisplayInfo, Osd};

pub(crate) struct Displays<T> {
    infos: Vec<DisplayInfo>,
    connections: Vec<Osd<T>>,
}

impl<T: DdcTransport> Displays<T> {
    pub(crate) fn new() -> Self {
        Self {
            infos: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub(crate) fn replace(&mut self, displays: Vec<(DisplayInfo, T)>) {
        let (infos, connections) = displays
            .into_iter()
            .map(|(info, display)| (info, Osd::new(display)))
            .unzip();
        self.infos = infos;
        self.connections = connections;
    }

    pub(crate) fn infos(&self) -> &[DisplayInfo] {
        &self.infos
    }

    pub(crate) fn display(&mut self, id: &str) -> Result<&mut Osd<T>, ApiError> {
        let index = exact_index(&self.infos, id)?;
        Ok(&mut self.connections[index])
    }
}

fn exact_index(infos: &[DisplayInfo], id: &str) -> Result<usize, ApiError> {
    let mut matches = infos.iter().enumerate().filter(|(_, info)| info.id() == id);
    let index = matches.next().map(|(index, _)| index).ok_or_else(|| {
        ApiError::new(
            crate::error::DISPLAY_NOT_FOUND,
            format!("display not found: {id}"),
        )
    })?;
    if matches.next().is_some() {
        return Err(ApiError::internal(format!("duplicate display ID: {id}")));
    }
    Ok(index)
}

#[cfg(test)]
mod tests;
