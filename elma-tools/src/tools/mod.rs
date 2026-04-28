mod evidence;
mod read;
mod respond;
mod search;
mod shell;
mod todo;
mod tool_search;

pub(crate) fn register_all(builder: &mut crate::registry::RegistryBuilder) {
    evidence::register(builder);
    read::register(builder);
    respond::register(builder);
    search::register(builder);
    shell::register(builder);
    todo::register(builder);
    tool_search::register(builder);
}
