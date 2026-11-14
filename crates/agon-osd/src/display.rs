#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayInfo {
    id: String,
    name: String,
    vendor_id: u32,
    model_id: u32,
    serial_number: u32,
    location: String,
}

impl DisplayInfo {
    pub fn new(
        id: String,
        name: String,
        vendor_id: u32,
        model_id: u32,
        serial_number: u32,
        location: String,
    ) -> Self {
        Self {
            id,
            name,
            vendor_id,
            model_id,
            serial_number,
            location,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn vendor_id(&self) -> u32 {
        self.vendor_id
    }

    pub fn model_id(&self) -> u32 {
        self.model_id
    }

    pub fn serial_number(&self) -> u32 {
        self.serial_number
    }

    pub fn location(&self) -> &str {
        &self.location
    }
}
