use cli_engine::NextAction;

pub(crate) fn next_action() {
    // Invoke the next action handler provided by cli_engine
    let _ = NextAction::execute();
}
