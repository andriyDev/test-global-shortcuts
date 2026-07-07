use dbus::{Message, Path, arg::Variant, blocking::Connection, message::SignalArgs};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    global_shortcuts::{
        OrgFreedesktopPortalGlobalShortcuts, OrgFreedesktopPortalGlobalShortcutsActivated,
    },
    registry::OrgFreedesktopHostPortalRegistry,
    request::OrgFreedesktopPortalRequestResponse,
};

#[allow(unused)]
mod global_shortcuts;
#[allow(unused)]
mod registry;
#[allow(unused)]
mod request;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // First open up a connection to the session bus.
    let conn = Connection::new_session()?;

    let proxy = conn.with_proxy(
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        Duration::from_millis(5000),
    );

    proxy.register("test_app", HashMap::new())?;

    println!("registered");

    let mut args = dbus::arg::PropMap::new();
    args.insert(
        "session_handle_token".into(),
        Variant(Box::new(String::from("test_session"))),
    );
    args.insert(
        "handle_token".into(),
        Variant(Box::new(String::from("test_request"))),
    );
    let request_handle = proxy.create_session(args)?;

    let result = Arc::new(Mutex::new(None));
    let result_clone = result.clone();
    conn.add_match(
        OrgFreedesktopPortalRequestResponse::match_rule(None, Some(&request_handle)).static_clone(),
        move |response: OrgFreedesktopPortalRequestResponse, _: &Connection, _: &Message| {
            *result_clone.lock().unwrap() = Some(response);
            false
        },
    )?;
    let result = loop {
        if let Some(result) = result.lock().unwrap().take() {
            break result;
        }
        conn.process(Duration::MAX).unwrap();
    };

    assert_eq!(result.response, 0, "details={:?}", result.results);

    let session_handle = result
        .results
        .get("session_handle")
        .unwrap()
        .0
        .as_str()
        .unwrap();
    let session_handle = Path::new(session_handle).unwrap();

    println!("created session");

    let mut shortcuts = vec![];
    shortcuts.push(("test-a", {
        let mut map = HashMap::new();
        map.insert(
            String::from("description"),
            Variant(Box::new(String::from("toggle mute")) as _),
        );
        map.insert(
            String::from("preferred_trigger"),
            Variant(Box::new(String::from("KP_Multiply")) as _),
        );
        map
    }));
    shortcuts.push(("test-b", {
        let mut map = HashMap::new();
        map.insert(
            String::from("description"),
            Variant(Box::new(String::from("toggle deafen")) as _),
        );
        map.insert(
            String::from("preferred_trigger"),
            Variant(Box::new(String::from("KP_Divide")) as _),
        );
        map
    }));

    let request_handle = proxy.bind_shortcuts(session_handle, shortcuts, "", Default::default())?;

    let result = Arc::new(Mutex::new(None));
    let result_clone = result.clone();
    conn.add_match(
        OrgFreedesktopPortalRequestResponse::match_rule(None, Some(&request_handle)).static_clone(),
        move |response: OrgFreedesktopPortalRequestResponse, _: &Connection, _: &Message| {
            *result_clone.lock().unwrap() = Some(response);
            false
        },
    )?;
    let result = loop {
        if let Some(result) = result.lock().unwrap().take() {
            break result;
        }
        conn.process(Duration::MAX).unwrap();
    };

    assert_eq!(result.response, 0, "details={:?}", result.results);

    let bound_shortcuts = result.results.get("shortcuts").unwrap();
    println!("bound shortcuts: {bound_shortcuts:?}");

    conn.add_match(
        OrgFreedesktopPortalGlobalShortcutsActivated::match_rule(None, None),
        |activated: OrgFreedesktopPortalGlobalShortcutsActivated, _: &Connection, _: &Message| {
            match activated.shortcut_id.as_str() {
                "test-a" => {
                    println!("triggered test-a");
                }
                "test-b" => {
                    println!("triggered test-b");
                }
                _ => {
                    println!("unknown");
                }
            }
            true
        },
    )?;

    loop {
        conn.process(Duration::MAX).unwrap();
    }
}
