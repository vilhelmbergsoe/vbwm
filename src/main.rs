use xcb::{x, Xid};

fn main() -> xcb::Result<()> {

    let (conn, screen_id) = xcb::Connection::connect(None)?;

    let setup = conn.get_setup();

    let screen = setup.roots().nth(screen_id as usize).unwrap();

    let window: x::Window = conn.generate_id();

    let cookie = conn.send_request_checked(&x::CreateWindow {
        depth: x::COPY_FROM_PARENT as u8,
        wid: window,
        parent: screen.root(),
        x: 0,
        y: 0,
        width: 150,
        height: 150,
        border_width: 0,
        class: x::WindowClass::InputOutput,
        visual: screen.root_visual(),
        // this list must be in same order than `Cw` enum order
        value_list: &[
            x::Cw::BackPixel(screen.black_pixel()),
            x::Cw::EventMask(x::EventMask::EXPOSURE | x::EventMask::KEY_PRESS)
        ],
    });

    conn.check_request(cookie)?;

    conn.send_request(&x::MapWindow {
        window,
    });

    let (wm_protocols, wm_del_window, wm_state, wm_state_maxv, wm_state_maxh) = {
        let cookies = (
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"WM_PROTOCOLS",
            }),
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"WM_DELETE_WINDOW",
            }),
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_STATE",
            }),
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_STATE_MAXIMIZED_VERT",
            }),
            conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"_NET_WM_STATE_MAXIMIZED_HORZ",
            }),
        );
        (
            conn.wait_for_reply(cookies.0)?.atom(),
            conn.wait_for_reply(cookies.1)?.atom(),
            conn.wait_for_reply(cookies.2)?.atom(),
            conn.wait_for_reply(cookies.3)?.atom(),
            conn.wait_for_reply(cookies.4)?.atom(),
        )
    };

    conn.check_request(conn.send_request_checked(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window,
        property: wm_protocols,
        r#type: x::ATOM_ATOM,
        data: &[wm_del_window],
    }))?;

    conn.flush()?;

    let mut maximized = false;

    loop {
        match conn.wait_for_event()? {
            xcb::Event::X(x::Event::KeyPress(ev)) => {
                if ev.detail() == 0x3a {
                    // The M key was pressed
                    // (M only on qwerty keyboards. Keymap support is done
                    // with the `xkb` extension and the `xkbcommon-rs` crate)

                    // We toggle maximized state, for this we send a message
                    // by building a `x::ClientMessageEvent` with the proper
                    // atoms and send it to the server.

                    let data = x::ClientMessageData::Data32([
                        if maximized { 0 } else { 1 },
                        wm_state_maxv.resource_id(),
                        wm_state_maxh.resource_id(),
                        0,
                        0,
                    ]);
                    let event = x::ClientMessageEvent::new(window, wm_state, data);
                    let cookie = conn.send_request_checked(&x::SendEvent {
                        propagate: false,
                        destination: x::SendEventDest::Window(screen.root()),
                        event_mask: x::EventMask::STRUCTURE_NOTIFY,
                        event: &event,
                    });
                    conn.check_request(cookie)?;

                    // Same as before, if we don't check for error, we have to flush
                    // the connection.
                    // conn.flush()?;

                    maximized = !maximized;
                } else if ev.detail() == 0x18 {
                    // Q (on qwerty)

                    // We exit the event loop (and the program)
                    break;
                }
            }
            xcb::Event::X(x::Event::ClientMessage(ev)) => {
                // We have received a message from the server
                if let x::ClientMessageData::Data32([atom, ..]) = ev.data() {
                    if atom == wm_del_window.resource_id() {
                        // The received atom is "WM_DELETE_WINDOW".
                        // We can check here if the user needs to save before
                        // exit, or in our case, exit right away.
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}
