use index3ds_common::*;
use serde::*;
use stdweb::web::*;
use yew::format::json::Json;
use yew::format::Nothing;
use yew::prelude::*;
use yew::services::fetch::*;
use yew::{html, Component, ComponentLink, Html, Renderable, ShouldRender};

use crate::language_map::*;

enum TableStatus {
    Loading,
    Error,
    Loaded(Vec<NcchInfo>),
}

pub struct PageNcchList {
    link: ComponentLink<PageNcchList>,
    fetch_service: FetchService,
    ncch_fetch_task: Option<FetchTask>,
    count_fetch_task: Option<FetchTask>,
    table_status: TableStatus,
    filter_param: NcchFilterParam,
    current_page: u32,
    total_page: Option<u32>,
    ncchs_in_page: u32,
    search_string: String,
}

pub enum Msg {
    PageChanged(u32),
    CountReceived(u32),
    NcchReceived(Vec<NcchInfo>),
    NcchError,
    UpdateSearchBox(String),
    Search,
    None,
}

impl PageNcchList {
    fn refresh_table(&mut self) {
        self.table_status = TableStatus::Loading;
        let param = NcchQueryParam {
            offset: (self.current_page * self.ncchs_in_page) as i64,
            limit: self.ncchs_in_page as i64,
            filter: self.filter_param.clone(),
        };
        let query = serde_urlencoded::ser::to_string(param).unwrap();
        let request = Request::get(&format!("{}?{}", url::query_ncch(), query))
            .body(Nothing)
            .unwrap();
        self.ncch_fetch_task = Some(self.fetch_service.fetch(
            request,
            self.link.send_back(|response: Response<_>| {
                let Json(body) = response.into_body();
                match body {
                    Ok(NcchQueryResponse::Ok(ncchs)) => Msg::NcchReceived(ncchs.ncchs),
                    _ => Msg::NcchError,
                }
            }),
        ));
    }

    fn refresh_page_selector(&mut self) {
        self.total_page = None;
        let query = serde_urlencoded::ser::to_string(&self.filter_param).unwrap();
        let request = Request::get(&format!("{}?{}", url::query_ncch_count(), query))
            .body(Nothing)
            .unwrap();
        self.count_fetch_task = Some(self.fetch_service.fetch(
            request,
            self.link.send_back(|response: Response<_>| {
                let Json(body) = response.into_body();
                match body {
                    Ok(NcchQueryCountResponse::Ok(response)) => {
                        Msg::CountReceived(response.count as u32)
                    }
                    _ => Msg::NcchError,
                }
            }),
        ));
    }
}

#[derive(Serialize, Deserialize, Properties, PartialEq, Clone)]
pub struct PageNcchListProp {
    #[serde(default)]
    #[props(required)]
    pub current_page: u32,
    #[serde(flatten)]
    #[props(required)]
    pub filter: NcchFilterParam,
}

impl Component for PageNcchList {
    type Message = Msg;
    type Properties = PageNcchListProp;

    fn create(props: Self::Properties, mut link: ComponentLink<Self>) -> Self {
        let search_string = props
            .filter
            .keyword
            .clone()
            .unwrap_or_else(|| "".to_owned());
        let mut component = PageNcchList {
            link,
            fetch_service: FetchService::new(),
            ncch_fetch_task: None,
            count_fetch_task: None,
            table_status: TableStatus::Loading,
            filter_param: props.filter,
            current_page: props.current_page,
            total_page: None,
            ncchs_in_page: 20,
            search_string,
        };
        component.refresh_table();
        component.refresh_page_selector();
        component
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::NcchReceived(ncchs) => {
                self.table_status = TableStatus::Loaded(ncchs);
            }
            Msg::CountReceived(count) => {
                self.total_page = Some((std::cmp::max(count, 1) - 1) / self.ncchs_in_page + 1);
            }
            Msg::NcchError => {
                self.table_status = TableStatus::Error;
            }
            Msg::PageChanged(page) => {
                self.current_page = page;
                self.refresh_table();
                self.push_history();
            }
            Msg::UpdateSearchBox(text) => {
                self.search_string = text;
            }
            Msg::Search => {
                if self.search_string.is_empty() {
                    self.filter_param.keyword = None;
                } else {
                    self.filter_param.keyword = Some(self.search_string.clone());
                }
                self.current_page = 0;
                self.refresh_table();
                self.refresh_page_selector();
                self.push_history();
            }
            Msg::None => {}
        }
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.current_page != props.current_page || self.filter_param != props.filter {
            self.current_page = props.current_page;
            self.filter_param = props.filter;
            self.refresh_table();
            self.refresh_page_selector();
            true
        } else {
            false
        }
    }
}

impl PageNcchList {
    fn push_history(&self) {
        let props = PageNcchListProp {
            current_page: self.current_page,
            filter: self.filter_param.clone(),
        };
        let search = serde_urlencoded::ser::to_string(props.clone()).unwrap();
        window()
            .history()
            .push_state((), "", Some(&format!("{}?{}", url::ncch_list(), search)))
    }

    fn view_page_selector(&self) -> Html<Self> {
        let loading = self.total_page.is_none();
        let total_page = self.total_page.unwrap_or(1);

        let range = if total_page < 5 {
            (0..total_page)
        } else if self.current_page < 2 {
            (0..5)
        } else if self.current_page + 3 > total_page {
            (total_page - 5..total_page)
        } else {
            (self.current_page - 2..self.current_page + 3)
        };

        let prev = std::cmp::max(self.current_page, 1) - 1;
        let next = std::cmp::min(self.current_page + 1, total_page - 1);

        let button_class = format!("button{}", if loading { "is-loading" } else { "" });

        html! {
            <div class="field has-addons">
                <p class="control">
                    <a class = button_class.as_str() onclick=|_| Msg::PageChanged(0)
                        disabled = {self.current_page == 0}>
                        <span class="icon">
                            <i class="fas fa-angle-double-left"/>
                        </span>
                    </a>
                </p>
                <p class="control">
                    <a class = button_class.as_str() onclick=|_| Msg::PageChanged(prev),
                        disabled = {self.current_page == 0}>
                        <span class="icon">
                            <i class="fas fa-angle-left"/>
                        </span>
                    </a>
                </p>

                {
                    for range.map(|i|html!{
                        <p class="control">
                            <a class = format!("{}{}", button_class, if self.current_page == i {" is-dark"} else {""})
                                onclick=|_| Msg::PageChanged(i)>
                            {format!("{}", i + 1)}
                            </a>
                        </p>
                    })
                }

                <p class="control">
                    <a class = button_class.as_str() onclick=|_| Msg::PageChanged(next)
                        disabled = {self.current_page == total_page - 1}>
                        <span class="icon">
                            <i class="fas fa-angle-right"/>
                        </span>
                    </a>
                </p>
                <p class="control">
                    <a class = button_class.as_str() onclick=|_| Msg::PageChanged(total_page - 1)
                        disabled = {self.current_page == total_page - 1}>
                        <span class="icon">
                            <i class="fas fa-angle-double-right"/>
                        </span>
                    </a>
                </p>
            </div>
        }
    }
}

impl Renderable<PageNcchList> for PageNcchList {
    fn view(&self) -> Html<Self> {
        html! {
            <div>
                <nav class="level">
                    <div class="level-left">
                        <div class="level-item">
                            { self.view_page_selector() }
                        </div>
                        <div class="level-item has-text-danger is-italic">
                            {"no, you can't download games here."}
                        </div>
                    </div>
                    <div class="level-right">
                        <div class="level-item">
                            <div class="field has-addons">
                                <p class="control has-icons-left">
                                    <input class="input" type="text"
                                        value=&self.search_string
                                        oninput=|e| Msg::UpdateSearchBox(e.value)
                                        onkeypress=|e| {
                                            if e.key() == "Enter" {
                                                Msg::Search
                                            } else {
                                                Msg::None
                                            }
                                        }
                                    />
                                    <span class="icon is-small is-left">
                                        <i class="fas fa-search"/>
                                    </span>
                                </p>
                                <p class="control">
                                    <button class="button" onclick=|_|Msg::Search>{"Search"}</button>
                                </p>
                            </div>
                        </div>
                    </div>
                </nav>
                {
                    if let TableStatus::Loaded(ncchs) = &self.table_status {
                        html!{
                            <table class="table is-striped is-narrow is-hoverable">
                                <thead>
                                    <tr>
                                        <th>{"Detail"}</th>
                                        <th>{"Icon"}</th>
                                        <th>{"Partition ID"}</th>
                                        <th>{"Program ID"}</th>
                                        <th>{"Product Code"}</th>
                                        <th>{"Title"}</th>
                                        <th>{"Publisher"}</th>
                                    </tr>
                                </thead>
                                <tbody> {for ncchs.iter().map(|ncch|{
                                    let has_smdh = ncch.short_title.is_some();
                                    let (title, publisher, icon) = if (has_smdh) {
                                        let region = ncch.region_lockout.unwrap();
                                        let index = if (region & (1 << 1)) != 0 {
                                            1
                                        } else {
                                            let region_bit = region.trailing_zeros() as usize;
                                            if region_bit >= LANGUAGE_MAP.len() {
                                                1
                                            } else {
                                                LANGUAGE_MAP[region_bit][0]
                                            }
                                        };
                                        (ncch.long_title.as_ref().unwrap()[index].as_str(),
                                         ncch.publisher.as_ref().unwrap()[index].as_str(),
                                         url::ncch_info(&ncch.id, "icon_small.png")
                                        )
                                    } else {
                                        ("", "", url::not_found_small().to_owned())
                                    };

                                    html!{<tr>
                                        <td><a href = format!("{}?{}", url::ncch(), ncch.id)>
                                            {"View"}
                                        </a></td>
                                        <td><img src=icon/></td>
                                        <td class="is-family-monospace">{&ncch.partition_id}</td>
                                        <td class="is-family-monospace">{&ncch.program_id}</td>
                                        <td class="is-family-monospace">{&ncch.product_code}</td>
                                        <td>{title}</td>
                                        <td>{publisher}</td>
                                    </tr>}
                                })} </tbody>
                            </table>
                        }
                    } else {
                        html!{
                            <div>{"..."}</div>
                        }
                    }
                }
            </div>
        }
    }
}
