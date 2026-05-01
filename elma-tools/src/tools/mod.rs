mod edit;
mod fetch;
mod glob;
mod ls;
mod observe;
pub(crate) mod patch;
mod read;
mod respond;
mod search;
mod shell;
mod summary;
mod todo;
mod tool_search;
mod write;

pub(crate) fn register_all(builder: &mut crate::registry::RegistryBuilder) {
    edit::register(builder);
    fetch::register(builder);
    glob::register(builder);
    ls::register(builder);
    observe::register(builder);
    patch::register(builder);
    read::register(builder);
    respond::register(builder);
    search::register(builder);
    shell::register(builder);
    summary::register(builder);
    todo::register(builder);
    tool_search::register(builder);
    write::register(builder);
}
