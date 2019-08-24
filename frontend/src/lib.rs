#![recursion_limit = "4096"]
use index3ds_common::*;
use stdweb::web::{*, event::PopStateEvent};
use yew::{html, Component, ComponentLink, Html, Renderable, ShouldRender};

mod language_map;
mod ncch;
mod ncch_list;
mod submit_ncch;

use ncch::PageNcch;
use ncch_list::PageNcchList;
use submit_ncch::PageSubmitNcch;

pub struct Model {
    burger_active: bool,
    history_listener: EventListenerHandle,
}

pub enum Msg {
    ToggleBurger,
    PopHistory,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, mut link: ComponentLink<Self>) -> Self {
        let pop_history = link.send_back(|_|Msg::PopHistory);
        let history_listener = window().add_event_listener(move |_: PopStateEvent| {
            pop_history.emit(());
        });
        Model {
            burger_active: false,
            history_listener,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ToggleBurger => self.burger_active = !self.burger_active,
            Msg::PopHistory => ()
        }
        true
    }
}

fn get_url() -> Option<(String, String)> {
    let location = window().location()?;
    Some((location.pathname().ok()?, location.search().ok()?))
}

impl Model {
    fn view_not_found(&self) -> Html<Self> {
        html! {
            <div>
                {"Page not found"}
            </div>
        }
    }
}

impl Model {
    fn view_content(&self) -> Html<Self> {
        if let Some((pathname, search)) = get_url() {
            if pathname == url::ncch() {
                if search.is_empty() {
                    self.view_not_found()
                } else {
                    html! {<PageNcch ncch_id = search[1..].to_owned()/>}
                }
            } else if pathname == url::ncch_list() {
                let search = if search.is_empty() {
                    &search
                } else {
                    &search[1..]
                };
                if let Ok(search) =
                    serde_urlencoded::de::from_str::<ncch_list::PageNcchListProp>(search)
                {
                    html! {<PageNcchList current_page=search.current_page filter=search.filter/>}
                } else {
                    self.view_not_found()
                }
            } else if pathname == url::submit_ncch() {
                html! {<PageSubmitNcch/>}
            } else {
                self.view_not_found()
            }
        } else {
            self.view_not_found()
        }
    }
}

impl Renderable<Model> for Model {
    fn view(&self) -> Html<Self> {
        let burger_active = if self.burger_active { " is-active" } else { "" };
        html! {
            <div>
                <nav class="navbar" role="navigation" aria-label="main navigation"> <div class="container">
                    <div class="navbar-brand">
                        <a class="navbar-item" href="/">
                        <img src="/logo.png" width="112" height="28"/>
                        </a>

                        <a role="button" class=format!("navbar-burger burger{}", burger_active)
                            aria-label="menu" aria-expanded="false" onclick=|_|Msg::ToggleBurger>
                            <span aria-hidden="true"/>
                            <span aria-hidden="true"/>
                            <span aria-hidden="true"/>
                        </a>
                    </div>
                    <div class=format!("navbar-menu{}", burger_active)>
                        <div class="navbar-start">
                            <a class="navbar-item" href=url::ncch_list()>
                                {"List"}
                            </a>
                            <a class="navbar-item" href=url::submit_ncch()>
                                {"Add"}
                            </a>
                        </div>
                    </div>
                </div> </nav>

                <div class="container">
                    { self.view_content() }
                </div>
            </div>
        }
    }
}
