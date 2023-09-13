use std::fmt;

pub enum Type {
    Major,
    Minor,
    Patch,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Major => write!(f, "Major"),
            Type::Minor => write!(f, "Minor"),
            Type::Patch => write!(f, "Patch"),
        }
    }
}
