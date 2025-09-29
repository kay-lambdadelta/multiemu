use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt::Debug};

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct AudioOutputId(pub Cow<'static, str>);

#[derive(Debug)]
pub struct AudioManager {
    
}
