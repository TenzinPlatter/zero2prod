use anyhow::Result;

use crate::{
    domain::{SubscriberEmail, subscriber_name::SubscriberName},
    routes::FormData,
};

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}

impl NewSubscriber {
    pub fn new(name: String, email: String) -> Result<NewSubscriber, String> {
        Ok(NewSubscriber {
            name: SubscriberName::parse(name).map_err(|e| e.to_string())?,
            email: SubscriberEmail::parse(email).map_err(|e| e.to_string())?,
        })
    }
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        NewSubscriber::new(value.name, value.email)
    }
}
