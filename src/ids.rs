use ulid::Ulid;

pub fn session_id() -> String {
    format!("sess_{}", Ulid::new())
}

pub fn turn_id() -> String {
    format!("turn_{}", Ulid::new())
}
