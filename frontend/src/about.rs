use yew::format::Nothing;
use yew::prelude::*;
use yew::services::fetch::*;

pub struct PageAbout {
    fetch_service: FetchService,
    fetch_task: FetchTask,
    git_revision: Option<String>,
}

pub enum Msg {
    GitRevision(String),
}

impl Component for PageAbout {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, mut link: ComponentLink<Self>) -> Self {
        let mut fetch_service = FetchService::new();
        let get_request = Request::get("/git-revision").body(Nothing).unwrap();
        let fetch_task = fetch_service.fetch(
            get_request,
            link.send_back(|response: Response<_>| match response.into_body() {
                Ok(s) => Msg::GitRevision(s),
                Err(_) => Msg::GitRevision("".to_owned()),
            }),
        );
        PageAbout {
            fetch_service,
            fetch_task,
            git_revision: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::GitRevision(s) => self.git_revision = Some(s),
        }
        true
    }
}

impl Renderable<PageAbout> for PageAbout {
    fn view(&self) -> Html<Self> {
        html! {<div>

        <section class="hero">
            <div class="hero-body">
                <div class="container">
                <h1 class="title">
                    {"Index3ds"}
                </h1>
                <h3 class="subtitle">
                    <span class="icon">
                        <i class="fas fa-code-branch"></i>
                    </span>
                    {self.git_revision.as_ref().map(|x|x.as_str()).unwrap_or("...")}
                </h3>
                </div>
            </div>
        </section>

        <section class="section">
        <div class = "content">
        <h2>{"What?"}</h2>
        <p>{"Index3ds is a database for 3DS games, or more accurately,
for all official 3DS packaged in the NCCH format. Everyone can add missing entries by
uploading (only a small part of) the game files they dumped."}</p>
        <h2>{"Why?"}</h2>
        <p>{"There are already some 3DS game databases.
        However, I want a database that does better than them in the following:"}
        <ul>
        <li>{"complete game database including all versions and regions,"}</li>
        <li>{"no piracy-related stuff,"}</li>
        <li>{"including useful technical information, and"}</li>
        <li>{"automatically verifying to avoid mistake made by people."}</li>
        </ul></p>
        <h2>{"How?"}</h2>
        <p>{"Instead of relying on information from those 'piracy releasing groups',
this database is build by everyone. Index3ds encourages everyone to dump games that they own legitimately,
and upload them. The server checks the signature built in the game to make sure it is intact official content,
then parses and records the game information, including ID and titles etc., to avoid human error.
"}</p>
        <h4>{"Uploading games? This website supports piracy?"}</h4>
        <p>{"No, index3ds doesn't support piracy. Only the necessary portion of the game
file is transferred through the internet and stored in the database. This portion only includes everything that
is shown in the game information page (or that will be shown soon, as some stuff is unimplemented right now), and
doesn't include the actual game playable content at all. I can't get your full game copy when you upload it,
nor anyone."}</p>
        <h4>{"What if I upload modified games or homebrew apps to pollute this database?"}</h4>
        <p>{"The server will reject any of such files. This is done by checking the hashes and signatures
in the game files that only official can sign. Stock 3DS have the same mechanic to prevent modified games or homebrew apps
from running (hacked 3DS simply have the check workarounded or disabled). I dumped the RSA public key from my 3DS, installed it
to the server, and let it do the same verification process. Database pollution can only happens when one of the following happens:
"}
        <ul>
        <li>{"the server is hacked,"}</li>
        <li>{"the RSA private key is leaked, or"}</li>
        <li>{"quantum computers become a real thing and the private key is cracked."}</li>
        </ul>
        </p>
        <p>{"The server only verifies signatues and hashes of the portion it stores, though.
If you upload a file that has the unstored part modified, it might pass the verification,
but the database is still fine as the stored information is verified."}</p>

        <h2>{"I want to contribute!"}</h2>
        <p>{"Thank you! As mentioned above, the simplest way to contribute is to upload more games and contents to this database.
        The goal of this database is to store information of all officially signed NCCH contents that ever existed."}</p>

        <p>{"You can also contribute by maintaining and improving the website itself.
        Index3ds is open-sourced and is hosted on Github and code contribution is welcome!"}<br/>
        <a class="button is-primary" href="https://github.com/wwylele/index3ds">
            <span class="icon">
            <i class="fab fa-github"></i>
            </span>
            <span>{"Visit index3ds on Github"}</span>
        </a>
        </p>
        <p>{"If you encountered any problem when using index3ds,
            please report as an issue to the Github repository. Problems include but not are limited to"}
            <ul>
            <li>{"the server is down,"}</li>
            <li>{"the web page is broken,"}</li>
            <li>{"you are not able to upload an official game that is legitimately dumped and unmodified, or"}</li>
            <li>{"you are able to upload an unofficial or modified game."}</li>
            </ul>
        </p>

        </div>
        </section>
        </div>
                }
    }
}
