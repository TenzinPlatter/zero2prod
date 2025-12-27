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
    pub fn new(name: String, email: String) -> Result<NewSubscriber> {
        Ok(NewSubscriber {
            name: SubscriberName::parse(name)?,
            email: SubscriberEmail::parse(email)?,
        })
    }
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = anyhow::Error;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        NewSubscriber::new(value.name, value.email)
    }
}
