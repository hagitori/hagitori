use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Chapter {
    pub id: String,
    pub number: String,
    pub name: String,
    pub title: Option<String>,
    pub date: Option<String>,
    pub scanlator: Option<String>,
}

impl Chapter {
    pub fn new(
        id: impl Into<String>,
        number: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            number: number.into(),
            name: name.into(),
            title: None,
            date: None,
            scanlator: None,
        }
    }

    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    #[must_use]
    pub fn with_scanlator(mut self, scanlator: impl Into<String>) -> Self {
        self.scanlator = Some(scanlator.into());
        self
    }

    #[must_use]
    pub fn with_date(mut self, date: impl Into<String>) -> Self {
        self.date = Some(date.into());
        self
    }
}
