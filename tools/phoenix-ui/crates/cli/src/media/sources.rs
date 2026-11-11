use super::{relevance, rights};
use image::ImageReader;
use regex::Regex;
use serde_json::{json, Value};
use std::io::{Cursor, Read};
use std::sync::LazyLock;

pub(super) const USER_AGENT: &str = concat!(
    "phoenix-ui-media/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/XGENT-ai/skills)"
);
const JSON_LIMIT: u64 = 16 * 1024 * 1024;

#[derive(Clone)]
pub(super) struct Endpoints {
    pub polyhaven: String,
    pub vam: String,
    pub wellcome: String,
    pub loc: String,
    pub nasa: String,
    pub commons: String,
    pub aic: String,
    pub cleveland: String,
    pub met: String,
    pub japan: String,
    pub rijks: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            polyhaven: "https://api.polyhaven.com".into(),
            vam: "https://api.vam.ac.uk".into(),
            wellcome: "https://api.wellcomecollection.org".into(),
            loc: "https://www.loc.gov".into(),
            nasa: "https://images-api.nasa.gov".into(),
            commons: "https://commons.wikimedia.org".into(),
            aic: "https://api.artic.edu".into(),
            cleveland: "https://openaccess-api.clevelandart.org".into(),
            met: "https://collectionapi.metmuseum.org".into(),
            japan: "https://jpsearch.go.jp".into(),
            rijks: "https://data.rijksmuseum.nl".into(),
        }
    }
}

#[derive(Clone)]
pub(super) struct Task {
    pub provider: &'static str,
    pub kind: &'static str,
    pub query: String,
}

pub(super) struct SearchResult {
    pub items: Vec<Value>,
    pub warnings: Vec<String>,
}

pub(super) fn tasks(kind: &str, query: &str) -> Vec<Task> {
    let pairs: &[(&str, &str)] = match kind {
        "texture" => &[("polyhaven", "texture")],
        "design" => &[
            ("met", "design"),
            ("aic", "design"),
            ("cleveland", "design"),
            ("vam", "design"),
            ("polyhaven", "model"),
            ("japan", "design"),
            ("rijks", "design"),
        ],
        "photo" => &[("commons", "photo"), ("loc", "photo"), ("nasa", "photo")],
        "blueprint" => &[("loc", "blueprint"), ("commons", "blueprint")],
        "map" => &[("loc", "map"), ("commons", "map")],
        "plate" => &[
            ("wellcome", "plate"),
            ("japan", "plate"),
            ("commons", "plate"),
        ],
        "science" => &[("nasa", "science"), ("wellcome", "science")],
        "video" => &[("nasa", "video")],
        _ => &[
            ("commons", "photo"),
            ("japan", "design"),
            ("polyhaven", "texture"),
            ("aic", "design"),
            ("loc", "photo"),
            ("rijks", "design"),
            ("cleveland", "design"),
            ("wellcome", "plate"),
            ("met", "design"),
            ("nasa", "science"),
            ("loc", "blueprint"),
            ("loc", "map"),
            ("vam", "design"),
            ("nasa", "video"),
            ("polyhaven", "model"),
        ],
    };
    pairs
        .iter()
        .map(|&(provider, kind)| Task {
            provider,
            kind,
            query: query.into(),
        })
        .collect()
}

pub(super) fn normalize_url(input: &str) -> Result<String, String> {
    let value = if input.starts_with("//") {
        format!("https:{input}")
    } else {
        input.to_owned()
    };
    let request = ureq::get(&value)
        .request_url()
        .map_err(|e| format!("无效素材地址：{e}"))?;
    let mut url = request.as_url().clone();
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host_str().is_none()
    {
        return Err(format!("素材地址必须是不含用户信息的 HTTP(S) URL：{input}"));
    }
    let host = url.host_str().unwrap_or("");
    let loopback = host == "localhost"
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    if url.scheme() == "http" && !loopback {
        url.set_scheme("https")
            .map_err(|()| "无法规范化素材地址".to_owned())?;
    }
    Ok(url.into())
}

fn request(agent: &ureq::Agent, url: &str) -> Result<ureq::Request, String> {
    Ok(agent
        .get(&normalize_url(url)?)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/json"))
}

fn response_json(request: ureq::Request) -> Result<Value, String> {
    let response = request.call().map_err(|error| match error {
        ureq::Error::Transport(_) => format!("网络连接失败：{error}"),
        _ => format!("信源请求失败：{error}"),
    })?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(JSON_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("响应读取失败：{e}"))?;
    if bytes.len() as u64 > JSON_LIMIT {
        return Err("信源响应超过 16 MB".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("信源响应不是有效 JSON：{e}"))
}

pub(super) fn get_json(agent: &ureq::Agent, url: &str) -> Result<Value, String> {
    response_json(request(agent, url)?)
}

fn linked_json(agent: &ureq::Agent, url: &str) -> Result<Value, String> {
    response_json(request(agent, url)?.set("Accept", "application/ld+json"))
}

fn api(
    agent: &ureq::Agent,
    base: &str,
    path: &str,
    params: &[(&str, String)],
    aic: bool,
) -> Result<Value, String> {
    let mut req = request(agent, &format!("{}{path}", base.trim_end_matches('/')))?;
    for (name, value) in params {
        req = req.query(name, value);
    }
    if aic {
        req = req.set("AIC-User-Agent", USER_AGENT);
    }
    response_json(req)
}

fn rows<'a>(data: &'a Value, pointer: &str) -> &'a [Value] {
    data.pointer(pointer)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn s(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
fn number(value: &Value) -> u64 {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0)
}
fn id(value: &Value) -> String {
    value.as_str().map(str::to_owned).unwrap_or_else(|| {
        if value.is_number() {
            value.to_string()
        } else {
            String::new()
        }
    })
}
fn joined(values: &[&Value]) -> String {
    values
        .iter()
        .map(|v| plain(v))
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}
fn array_text(value: &Value) -> String {
    value
        .as_array()
        .map(|a| a.iter().map(plain).collect::<Vec<_>>().join(", "))
        .unwrap_or_else(|| plain(value))
}
static TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]*>").expect("valid tag pattern"));
fn plain(value: &Value) -> String {
    let text = s(value).replace("&lt;", "<").replace("&gt;", ">");
    TAGS.replace_all(&text, " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn iiif_base(url: &str) -> &str {
    let trimmed = url
        .split("/full/")
        .next()
        .unwrap_or(url)
        .split("/info.json")
        .next()
        .unwrap_or(url);
    trimmed.trim_end_matches('/')
}
#[expect(
    clippy::too_many_arguments,
    reason = "The shared wire-record constructor keeps URL and license fields explicit instead of grouping them into positional tuples."
)]
fn item(
    provider: &str,
    source_id: String,
    kind: &str,
    title: String,
    detail: String,
    page: String,
    image: String,
    preview: String,
    license: String,
    license_url: String,
    attribution: String,
) -> Value {
    let grade = rights::use_grade(&license, &license_url);
    json!({"ref":"", "provider":provider,"sourceId":source_id,"kind":kind,"title":title,"detail":detail,"pageUrl":page,"imageUrl":image,"previewUrl":preview,"license":license,"licenseUrl":license_url,"attribution":attribution,"use":grade,"useLabel":rights::use_label(grade),"originalWidth":0,"originalHeight":0})
}
fn dimensions(item: &mut Value, width: u64, height: u64) {
    item["originalWidth"] = width.into();
    item["originalHeight"] = height.into();
}

fn parse_polyhaven(data: &Value, kind: &str) -> Vec<Value> {
    data.as_object()
        .into_iter()
        .flat_map(|map| map.iter())
        .filter(|(slug, _)| !slug.starts_with('_'))
        .map(|(slug, meta)| {
            let preview =
                format!("https://cdn.polyhaven.com/asset_img/thumbs/{slug}.png?width=768");
            let authors = meta["authors"]
                .as_object()
                .map(|a| a.keys().cloned().collect::<Vec<_>>().join(", "))
                .unwrap_or_default();
            let mut out = item(
                "polyhaven",
                slug.clone(),
                kind,
                s(&meta["name"]).to_owned(),
                format!(
                    "{} {} {} {}",
                    slug.replace('_', " "),
                    array_text(&meta["categories"]),
                    array_text(&meta["tags"]),
                    s(&meta["description"])
                ),
                format!("https://polyhaven.com/a/{slug}"),
                preview.clone(),
                preview,
                "CC0".into(),
                "https://polyhaven.com/license".into(),
                format!("{authors} · Poly Haven"),
            );
            dimensions(
                &mut out,
                number(&meta["max_resolution"][0]),
                number(&meta["max_resolution"][1]),
            );
            out
        })
        .collect()
}

fn parse_vam(data: &Value) -> Vec<Value> {
    rows(data,"/records").iter().filter_map(|r| {
        let base = iiif_base(s(&r["_images"]["_iiif_image_base_url"]));
        if base.is_empty() { return None; }
        let source = id(&r["systemNumber"]);
        let mut out = item("vam", source.clone(), "design", plain(&r["_primaryTitle"]), joined(&[&r["objectType"],&r["_primaryMaker"]["name"],&r["_primaryDate"],&r["_primaryPlace"]]), format!("https://collections.vam.ac.uk/item/{source}"), format!("{base}/full/!1600,1600/0/default.jpg"), format!("{base}/full/!400,400/0/default.jpg"), "V&A terms: permitted non-commercial uses only; commercial/home-page/high-resolution publication requires licensing".into(), "https://www.vam.ac.uk/info/va-websites-terms-conditions".into(), format!("© Victoria and Albert Museum, London ({source})"));
        out["iiifInfoUrl"] = format!("{base}/info.json").into(); Some(out)
    }).collect()
}

fn parse_wellcome(data: &Value, kind: &str) -> Vec<Value> {
    rows(data, "/results")
        .iter()
        .filter_map(|r| {
            let location = rows(r, "/locations")
                .iter()
                .find(|loc| loc["locationType"]["id"] == "iiif-image")?;
            let base = iiif_base(s(&location["url"]));
            if base.is_empty() {
                return None;
            }
            let people = rows(r, "/source/contributors")
                .iter()
                .map(|c| plain(&c["agent"]["label"]))
                .collect::<Vec<_>>()
                .join(", ");
            let page = format!(
                "https://wellcomecollection.org/works/{}",
                s(&r["source"]["id"])
            );
            let license = location["license"]["label"]
                .as_str()
                .unwrap_or("Rights not specified; check item");
            let mut out = item(
                "wellcome",
                id(&r["id"]),
                kind,
                plain(&r["source"]["title"]),
                people.clone(),
                page.clone(),
                format!("{base}/full/!1600,1600/0/default.jpg"),
                format!("{base}/full/!400,400/0/default.jpg"),
                license.into(),
                location["license"]["url"].as_str().unwrap_or(&page).into(),
                format!(
                    "{people} · {}",
                    location["credit"].as_str().unwrap_or("Wellcome Collection")
                ),
            );
            out["iiifInfoUrl"] = format!("{base}/info.json").into();
            Some(out)
        })
        .collect()
}

fn parse_loc(data: &Value, kind: &str) -> Vec<Value> {
    rows(data, "/results")
        .iter()
        .filter(|r| r["access_restricted"] != true)
        .filter_map(|r| {
            let urls: Vec<_> = rows(r, "/image_url")
                .iter()
                .filter_map(Value::as_str)
                .filter(|url| {
                    let lower = url
                        .split(['?', '#'])
                        .next()
                        .unwrap_or(url)
                        .to_ascii_lowercase();
                    lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".png")
                })
                .collect();
            let (image, preview, page) = if kind == "blueprint" {
                let resource = rows(r, "/resources")
                    .iter()
                    .find(|resource| s(&resource["image"]).contains("/sheet/"));
                let raw = resource
                    .and_then(|r| r["image"].as_str())
                    .or_else(|| urls.iter().copied().find(|url| url.contains("/sheet/")))?;
                let image = raw.replace("_150px.jpg", "v.jpg").replace("r.jpg", "v.jpg");
                let page = resource
                    .and_then(|r| r["url"].as_str())
                    .or_else(|| r["url"].as_str())
                    .unwrap_or(s(&r["id"]));
                (image, raw.replace("_150px.jpg", "r.jpg"), page.to_owned())
            } else {
                let image = urls.last()?.split('#').next().unwrap_or("").to_owned();
                let preview = urls
                    .get(1)
                    .or_else(|| urls.first())?
                    .split('#')
                    .next()
                    .unwrap_or("")
                    .to_owned();
                (
                    image,
                    preview,
                    r["url"].as_str().unwrap_or(s(&r["id"])).to_owned(),
                )
            };
            let rights = [
                &r["rights_advisory"],
                &r["rights_information"],
                &r["item"]["rights_advisory"],
                &r["item"]["rights_information"],
                &r["rights"],
            ]
            .into_iter()
            .find(|v| !v.is_null());
            let license = rights.map(array_text).unwrap_or_else(|| {
                "Rights not specified in search; check Rights & Access on item page".into()
            });
            Some(item(
                "loc",
                id(&r["id"]),
                kind,
                plain(&r["title"]),
                format!("{} {}", plain(&r["date"]), array_text(&r["contributor"])),
                page.clone(),
                image,
                preview,
                license,
                page,
                r["item"]["repository"]
                    .as_str()
                    .unwrap_or("Library of Congress")
                    .into(),
            ))
        })
        .collect()
}

fn parse_nasa(data: &Value, kind: &str) -> Vec<Value> {
    static SYNTHETIC: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new("(?i)animation|visualization|simulation|computer.generated|rendering")
            .expect("valid video filter")
    });
    static COPYRIGHT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new("(?i)copyright|©|all rights reserved").expect("valid rights filter")
    });
    rows(data,"/collection/items").iter().filter_map(|row| {
        let r = rows(row,"/data").first()?;
        if kind == "video" && SYNTHETIC.is_match(&format!("{} {}",s(&r["title"]),array_text(&r["keywords"]))) { return None; }
        let image = rows(row,"/links").iter().find(|link| link["render"] == "image" && link["rel"] == "preview").or_else(||rows(row,"/links").iter().find(|link| link["render"] == "image"))?["href"].as_str()?;
        let third_party = COPYRIGHT.is_match(&joined(&[&r["description"],&r["photographer"],&r["secondary_creator"]]));
        let license = if third_party { "Third-party rights mentioned; check item credits" } else { "NASA media usage guidelines; generally not subject to US copyright; verify third-party credits" };
        let source = id(&r["nasa_id"]);
        let mut out = item("nasa",source.clone(),kind,plain(&r["title"]),joined(&[&r["center"],&r["date_created"],&r["description"]]),format!("https://images.nasa.gov/details/{source}"),image.into(),image.into(),license.into(),"https://www.nasa.gov/nasa-brand-center/images-and-media/".into(),format!("NASA · {}",joined(&[&r["center"],&r["photographer"],&r["secondary_creator"]])));
        out["manifestUrl"] = row["href"].clone(); out["video"] = (kind == "video").into(); Some(out)
    }).collect()
}

fn parse_commons(data: &Value, kind: &str) -> Vec<Value> {
    let mut pages = rows(data, "/query/pages").iter().collect::<Vec<_>>();
    pages.sort_by_key(|page| number(&page["index"]));
    pages
        .into_iter()
        .filter_map(|p| {
            let ii = rows(p, "/imageinfo").first()?;
            let meta = |name: &str| plain(&ii["extmetadata"][name]["value"]);
            let about = format!(
                "{} {} {}",
                s(&p["title"]),
                meta("ObjectName"),
                meta("ImageDescription")
            );
            let medium = match kind {
                "map" => Some(r"(?i)\b(map|maps|chart|plan)\b"),
                "blueprint" => {
                    Some(r"(?i)habs|haer|measured drawing|blueprint|elevation|floor plan|section")
                }
                "plate" => Some(r"(?i)engraving|etching|woodcut|lithograph|plate"),
                _ => None,
            };
            if medium.is_some_and(|pattern| {
                Regex::new(pattern).is_ok_and(|regex| !regex.is_match(&about))
            }) {
                return None;
            }
            let safe = matches!(
                s(&ii["mime"]),
                "image/jpeg" | "image/png" | "image/webp" | "image/gif"
            );
            let image = if number(&ii["width"]) > 1920 || !safe {
                s(&ii["thumburl"])
            } else {
                s(&ii["url"])
            };
            if image.is_empty() {
                return None;
            }
            let preview = if number(&ii["width"]) > 500 && !s(&ii["thumburl"]).is_empty() {
                static WIDTH: LazyLock<Regex> =
                    LazyLock::new(|| Regex::new(r"/\d+px-").expect("valid thumbnail pattern"));
                WIDTH.replace(s(&ii["thumburl"]), "/500px-").into_owned()
            } else {
                image.into()
            };
            let license = [meta("LicenseShortName"), meta("License")]
                .into_iter()
                .find(|v| !v.is_empty())
                .unwrap_or_else(|| "License on file page".into());
            let license_url = meta("LicenseUrl");
            let title = meta("ObjectName");
            let mut out = item(
                "commons",
                id(&p["pageid"]),
                kind,
                if title.is_empty() {
                    s(&p["title"]).trim_start_matches("File:").into()
                } else {
                    title
                },
                format!("{} {}", meta("Artist"), meta("ImageDescription")),
                s(&ii["descriptionurl"]).into(),
                image.into(),
                preview,
                license,
                if license_url.is_empty() {
                    s(&ii["descriptionurl"]).into()
                } else {
                    license_url
                },
                format!("{} · via Wikimedia Commons", meta("Artist")),
            );
            dimensions(&mut out, number(&ii["width"]), number(&ii["height"]));
            Some(out)
        })
        .collect()
}

fn parse_aic(data: &Value) -> Vec<Value> {
    let iiif = data["config"]["iiif_url"]
        .as_str()
        .unwrap_or("https://www.artic.edu/iiif/2");
    rows(data, "/data")
        .iter()
        .filter(|r| r["is_public_domain"] == true && !s(&r["image_id"]).is_empty())
        .map(|r| {
            let source = id(&r["id"]);
            let base = format!("{}/{}/full", iiif.trim_end_matches('/'), s(&r["image_id"]));
            let width = number(&r["thumbnail"]["width"]);
            let mut out = item(
                "aic",
                source.clone(),
                "design",
                plain(&r["title"]),
                joined(&[
                    &r["artist_display"],
                    &r["date_display"],
                    &r["thumbnail"]["alt_text"],
                ]),
                format!("https://www.artic.edu/artworks/{source}"),
                format!("{base}/{},/0/default.jpg", width.min(1686)),
                format!("{base}/400,/0/default.jpg"),
                "CC0 (public-domain artwork, Art Institute of Chicago)".into(),
                "https://www.artic.edu/open-access/open-access-images".into(),
                format!("Art Institute of Chicago ({source})"),
            );
            dimensions(&mut out, width, number(&r["thumbnail"]["height"]));
            out
        })
        .collect()
}

fn parse_cleveland(data: &Value) -> Vec<Value> {
    rows(data, "/data")
        .iter()
        .filter_map(|r| {
            let web = &r["images"]["web"];
            let print = if r["images"]["print"].is_object() {
                &r["images"]["print"]
            } else {
                web
            };
            let full = if r["images"]["full"].is_object() {
                &r["images"]["full"]
            } else {
                print
            };
            if s(&web["url"]).is_empty() {
                return None;
            }
            let mut out = item(
                "cleveland",
                id(&r["id"]),
                "design",
                plain(&r["title"]),
                format!(
                    "{} {}",
                    joined(&[
                        &r["type"],
                        &r["creation_date"],
                        &r["description"],
                        &r["technique"]
                    ]),
                    rows(r, "/creators")
                        .iter()
                        .map(|c| plain(&c["description"]))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                s(&r["url"]).into(),
                s(&print["url"]).into(),
                s(&web["url"]).into(),
                "CC0".into(),
                "https://www.clevelandart.org/open-access".into(),
                format!("Cleveland Museum of Art · {}", s(&r["creditline"])),
            );
            dimensions(&mut out, number(&full["width"]), number(&full["height"]));
            Some(out)
        })
        .collect()
}

fn parse_met(data: &Value) -> Vec<Value> {
    if data["isPublicDomain"] != true || s(&data["primaryImage"]).is_empty() {
        return vec![];
    }
    let source = id(&data["objectID"]);
    vec![item(
        "met",
        source.clone(),
        "design",
        plain(&data["title"]),
        joined(&[
            &data["objectName"],
            &data["artistDisplayName"],
            &data["objectDate"],
            &data["culture"],
            &data["medium"],
        ]),
        s(&data["objectURL"]).into(),
        s(&data["primaryImage"]).into(),
        data["primaryImageSmall"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(s(&data["primaryImage"]))
            .into(),
        "CC0 (Met Open Access, public domain)".into(),
        "https://www.metmuseum.org/policies/image-resources".into(),
        format!("The Metropolitan Museum of Art ({source})"),
    )]
}

fn parse_japan(data: &Value, kind: &str) -> Vec<Value> {
    rows(data, "/list")
        .iter()
        .filter_map(|r| {
            let c = &r["common"];
            let license = match s(&c["contentsRightsType"]) {
                "cc0" => "CC0",
                "pdm" => "Public Domain Mark",
                "ccby" => "CC BY 4.0",
                "ccbysa" => "CC BY-SA 4.0",
                _ => return None,
            };
            let raw = rows(c, "/contentsUrl").first()?.as_str()?;
            let large = raw.replace("/slideshow_s/", "/slideshow/");
            let source = c["id"].as_str().or_else(|| r["id"].as_str())?;
            let people = array_text(&c["contributor"]);
            let attribution = if c["database"] == "cobas" {
                format!("出典：ColBase（https://colbase.nich.go.jp/） · {people}")
            } else {
                format!("{people} · {} · via Japan Search", s(&c["provider"]))
            };
            let mut out = item(
                "japan",
                source.into(),
                kind,
                plain(&c["title"]),
                format!(
                    "{} {} {}",
                    joined(&[&c["titleEn"], &c["description"]]),
                    array_text(&c["temporal"]),
                    people
                ),
                c["linkUrl"]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("https://jpsearch.go.jp/item/{source}")),
                large.clone(),
                rows(c, "/thumbnailUrl")
                    .first()
                    .and_then(Value::as_str)
                    .unwrap_or(raw)
                    .into(),
                license.into(),
                "https://jpsearch.go.jp/policy/available-rights-statements".into(),
                attribution,
            );
            out["downloadCandidates"] = json!([large, raw]);
            Some(out)
        })
        .collect()
}

fn parse_rijks(data: &Value) -> Vec<Value> {
    let object = &data["object"];
    let digital = &data["digital"];
    let Some(access) = rows(digital, "/access_point")
        .first()
        .and_then(|v| v["id"].as_str())
    else {
        return vec![];
    };
    let service = iiif_base(access);
    let page = s(&object["id"]);
    let names: Vec<_> = rows(object, "/identified_by")
        .iter()
        .filter(|n| n["type"] == "Name")
        .collect();
    let name = names
        .iter()
        .find(|n| n["language"].to_string().contains("300388277"))
        .or_else(|| names.first());
    let number = rows(object, "/identified_by")
        .iter()
        .find(|n| n["type"] == "Identifier")
        .map(|n| s(&n["content"]))
        .unwrap_or("");
    static RIGHTS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"https?://creativecommons\.org/[^\s\"<>]+"#).expect("valid rights URL pattern")
    });
    let subject = object["subject_of"].to_string();
    let rights = RIGHTS.find(&subject).map(|m| m.as_str());
    let license = rights
        .map(|r| {
            if r.contains("/zero/") {
                "CC0"
            } else if r.contains("/mark/") {
                "Public Domain Mark"
            } else {
                r
            }
        })
        .unwrap_or("Rights not specified in record; check item page");
    let mut out = item(
        "rijks",
        page.rsplit('/').next().unwrap_or(page).into(),
        "design",
        name.map(|n| s(&n["content"])).unwrap_or(number).into(),
        format!(
            "{} {}",
            number,
            rows(object, "/referred_to_by")
                .iter()
                .map(|r| plain(&r["content"]))
                .collect::<Vec<_>>()
                .join(" · ")
        ),
        page.into(),
        format!("{service}/full/!2400,2400/0/default.jpg"),
        format!("{service}/full/!500,500/0/default.jpg"),
        license.into(),
        rights.unwrap_or(page).into(),
        format!("Rijksmuseum, Amsterdam ({number})"),
    );
    out["iiifInfoUrl"] = format!("{service}/info.json").into();
    vec![out]
}

pub(super) fn select_map(files: &Value, keys: &[&str], resolution: &str) -> Option<Value> {
    let requested = resolution.trim_end_matches('k').parse::<u32>().ok()?;
    for key in keys {
        let Some(group) = files[*key].as_object() else {
            continue;
        };
        let mut available: Vec<_> = group
            .iter()
            .filter_map(|(name, value)| {
                (!s(&value["jpg"]["url"]).is_empty())
                    .then(|| {
                        name.trim_end_matches('k')
                            .parse::<u32>()
                            .ok()
                            .map(|size| (size, name, value))
                    })
                    .flatten()
            })
            .collect();
        available.sort_by_key(|entry| entry.0);
        let selected = available
            .iter()
            .rev()
            .find(|entry| entry.0 <= requested)
            .or_else(|| available.first());
        if let Some((_, name, value)) = selected {
            let mut output = value["jpg"].clone();
            output["resolution"] = (*name).clone().into();
            return Some(output);
        }
    }
    None
}

fn header_dimensions(agent: &ureq::Agent, url: &str) -> Result<(u32, u32), String> {
    let response = request(agent, url)?
        .set("Accept", "image/*")
        .set("Range", "bytes=0-65535")
        .call()
        .map_err(|error| match error {
            ureq::Error::Transport(_) => format!("网络连接失败：{error}"),
            _ => format!("图片头部请求失败：{error}"),
        })?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(65536)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("图片头部读取失败：{e}"))?;
    ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .into_dimensions()
        .map_err(|e| format!("无法从图片头部取得原图尺寸：{e}"))
}

fn complete_item(
    agent: &ureq::Agent,
    endpoints: &Endpoints,
    mut item: Value,
) -> Result<Value, String> {
    for name in [
        "pageUrl",
        "imageUrl",
        "previewUrl",
        "licenseUrl",
        "manifestUrl",
        "iiifInfoUrl",
    ] {
        if let Some(url) = item[name].as_str().filter(|s| !s.is_empty()) {
            item[name] = normalize_url(url)?.into();
        }
    }
    if let Some(urls) = item["downloadCandidates"].as_array_mut() {
        for url in urls {
            *url = normalize_url(s(url))?.into();
        }
    }
    if s(&item["sourceId"]).is_empty()
        || s(&item["imageUrl"]).is_empty()
        || s(&item["previewUrl"]).is_empty()
    {
        return Err("条目缺少编号或图片地址".into());
    }
    let provider = s(&item["provider"]).to_owned();
    if provider == "polyhaven" {
        let files = get_json(
            agent,
            &format!(
                "{}/files/{}",
                endpoints.polyhaven.trim_end_matches('/'),
                s(&item["sourceId"])
            ),
        )?;
        if item["kind"] == "texture" {
            let color = select_map(&files, &["Diffuse", "diff", "Color", "col"], "1k")
                .ok_or("没有可用颜色贴图")?;
            item["textureMaps"] = json!({"color":color,"roughness":select_map(&files,&["Rough","rough","Roughness"],"1k"),"normal":select_map(&files,&["nor_gl","Normal","normal"],"1k")});
        }
    }
    if let Some(url) = item["iiifInfoUrl"].as_str() {
        let info = get_json(agent, url)?;
        dimensions(&mut item, number(&info["width"]), number(&info["height"]));
    } else if provider == "nasa" {
        let manifest = s(&item["manifestUrl"]);
        let metadata_url = format!(
            "{}/metadata.json",
            manifest
                .rsplit_once('/')
                .map(|(base, _)| base)
                .unwrap_or(manifest)
        );
        let metadata = get_json(agent, &metadata_url)?;
        let group = if item["video"] == true {
            "QuickTime"
        } else {
            "File"
        };
        dimensions(
            &mut item,
            number(&metadata[format!("{group}:ImageWidth")]),
            number(&metadata[format!("{group}:ImageHeight")]),
        );
    } else if matches!(provider.as_str(), "met" | "japan" | "loc") {
        let url = if provider == "japan" {
            item["downloadCandidates"]
                .as_array()
                .and_then(|urls| urls.last())
                .and_then(Value::as_str)
                .unwrap_or(s(&item["imageUrl"]))
        } else {
            s(&item["imageUrl"])
        };
        let (width, height) = header_dimensions(agent, url)?;
        dimensions(&mut item, width.into(), height.into());
    }
    if number(&item["originalWidth"]) == 0 || number(&item["originalHeight"]) == 0 {
        return Err("无法取得原图尺寸，已丢弃条目".into());
    }
    Ok(item)
}

pub(super) fn search(
    agent: &ureq::Agent,
    endpoints: &Endpoints,
    task: &Task,
    limit: usize,
) -> Result<SearchResult, String> {
    let query = &task.query;
    let count = |multiple: usize, maximum: usize| (limit * multiple).min(maximum).to_string();
    let mut warnings = Vec::new();
    let raw = match task.provider {
        "polyhaven" => {
            let kind = if task.kind == "texture" {
                "textures"
            } else {
                "models"
            };
            let data = api(
                agent,
                &endpoints.polyhaven,
                "/assets",
                &[("t", kind.into())],
                false,
            )?;
            let terms = relevance::query_terms(query);
            let mut items: Vec<_> = parse_polyhaven(&data, task.kind)
                .into_iter()
                .filter_map(|item| {
                    let score = relevance::match_count(
                        &format!("{} {}", s(&item["title"]), s(&item["detail"])),
                        &terms,
                    );
                    (score >= terms.len().div_ceil(2).max(1)).then_some((score, item))
                })
                .collect();
            items.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
            items
                .into_iter()
                .take(limit)
                .map(|(_, item)| item)
                .collect()
        }
        "vam" => parse_vam(&api(
            agent,
            &endpoints.vam,
            "/v2/objects/search",
            &[
                ("q", query.clone()),
                ("images_exist", "1".into()),
                ("page_size", limit.to_string()),
            ],
            false,
        )?),
        "wellcome" => parse_wellcome(
            &api(
                agent,
                &endpoints.wellcome,
                "/catalogue/v2/images",
                &[
                    ("query", query.clone()),
                    ("pageSize", limit.to_string()),
                    ("include", "source.contributors".into()),
                ],
                false,
            )?,
            task.kind,
        ),
        "loc" => {
            let path = match task.kind {
                "blueprint" => {
                    "/collections/historic-american-buildings-landscapes-and-engineering-records/"
                }
                "map" => "/maps/",
                _ => "/photos/",
            };
            let mut params = vec![
                (
                    "q",
                    if task.kind == "blueprint" {
                        format!("{query} drawings")
                    } else {
                        query.clone()
                    },
                ),
                ("fo", "json".into()),
                ("c", count(if task.kind == "blueprint" { 3 } else { 2 }, 60)),
            ];
            if task.kind == "blueprint" {
                params.push(("at", "results".into()));
            }
            parse_loc(
                &api(agent, &endpoints.loc, path, &params, false)?,
                task.kind,
            )
            .into_iter()
            .take(limit)
            .collect()
        }
        "nasa" => parse_nasa(
            &api(
                agent,
                &endpoints.nasa,
                "/search",
                &[
                    ("q", query.clone()),
                    (
                        "media_type",
                        if task.kind == "video" {
                            "video"
                        } else {
                            "image"
                        }
                        .into(),
                    ),
                    (
                        "page_size",
                        if task.kind == "video" {
                            count(3, 100)
                        } else {
                            limit.to_string()
                        },
                    ),
                ],
                false,
            )?,
            task.kind,
        )
        .into_iter()
        .take(limit)
        .collect(),
        "commons" => {
            let query = match task.kind {
                "map" => format!("{query} (map OR chart OR plan)"),
                "blueprint" => {
                    format!("{query} (HABS OR HAER OR \"measured drawing\" OR blueprint)")
                }
                "plate" => format!("{query} engraving"),
                _ => query.clone(),
            };
            parse_commons(&api(agent,&endpoints.commons,"/w/api.php",&[("action","query".into()),("format","json".into()),("formatversion","2".into()),("generator","search".into()),("gsrsearch",format!("{query} filetype:bitmap")),("gsrnamespace","6".into()),("gsrlimit",count(2,40)),("prop","imageinfo".into()),("iiprop","url|size|mime|extmetadata".into()),("iiurlwidth","1920".into()),("iiextmetadatafilter","LicenseShortName|License|LicenseUrl|Artist|ObjectName|ImageDescription|AttributionRequired".into())],false)?,task.kind)
        }
        "aic" => parse_aic(&api(
            agent,
            &endpoints.aic,
            "/api/v1/artworks/search",
            &[
                ("q", query.clone()),
                ("limit", count(2, 40)),
                (
                    "fields",
                    "id,title,artist_display,date_display,image_id,is_public_domain,thumbnail"
                        .into(),
                ),
                ("query[term][is_public_domain]", "true".into()),
            ],
            true,
        )?),
        "cleveland" => parse_cleveland(&api(
            agent,
            &endpoints.cleveland,
            "/api/artworks/",
            &[
                ("q", query.clone()),
                ("has_image", "1".into()),
                ("cc0", "1".into()),
                ("limit", count(2, 40)),
            ],
            false,
        )?),
        "met" => {
            let data = api(
                agent,
                &endpoints.met,
                "/public/collection/v1.1/search",
                &[
                    ("q", query.clone()),
                    ("hasImages", "true".into()),
                    ("isPublicDomain", "true".into()),
                    ("limit", count(2, 12)),
                ],
                false,
            )?;
            let mut items = Vec::new();
            for object_id in rows(&data, "/objectIDs").iter().take(limit * 2) {
                let url = format!(
                    "{}/public/collection/v1/objects/{}",
                    endpoints.met.trim_end_matches('/'),
                    id(object_id)
                );
                match get_json(agent, &url) {
                    Ok(object) => items.extend(parse_met(&object)),
                    Err(error) => warnings.push(format!("{}: {error}", id(object_id))),
                }
            }
            items
        }
        "japan" => parse_japan(
            &api(
                agent,
                &endpoints.japan,
                "/api/item/search/jps-cross",
                &[
                    ("keyword", query.clone()),
                    ("f-contents", "image".into()),
                    ("size", count(3, 60)),
                    ("f-rights", "+cc0".into()),
                    ("f-rights", "+pdm".into()),
                    ("f-rights", "+ccby".into()),
                    ("f-rights", "+ccbysa".into()),
                ],
                false,
            )?,
            task.kind,
        ),
        "rijks" => {
            let data = response_json(
                request(
                    agent,
                    &format!(
                        "{}/search/collection",
                        endpoints.rijks.trim_end_matches('/')
                    ),
                )?
                .set("Accept", "application/ld+json")
                .query("description", query)
                .query("imageAvailable", "true"),
            )?;
            let mut items = Vec::new();
            for result in rows(&data, "/orderedItems")
                .iter()
                .take((limit * 2).min(10))
            {
                let load = || -> Result<Vec<Value>, String> {
                    let object = linked_json(agent, s(&result["id"]))?;
                    let visual_url = rows(&object, "/shows")
                        .first()
                        .and_then(|v| v["id"].as_str())
                        .ok_or("馆藏没有 shows 图像记录")?;
                    let visual = linked_json(agent, visual_url)?;
                    let digital_url = rows(&visual, "/digitally_shown_by")
                        .first()
                        .and_then(|v| v["id"].as_str())
                        .ok_or("馆藏没有数字图像记录")?;
                    let digital = linked_json(agent, digital_url)?;
                    Ok(parse_rijks(&json!({"object":object,"digital":digital})))
                };
                match load() {
                    Ok(found) => items.extend(found),
                    Err(error) => warnings.push(format!("{}: {error}", s(&result["id"]))),
                }
            }
            items
        }
        other => return Err(format!("未知信源：{other}")),
    };
    let had_items = !raw.is_empty();
    let mut items = Vec::new();
    for item in raw {
        let source_id = s(&item["sourceId"]).to_owned();
        match complete_item(agent, endpoints, item) {
            Ok(item) => items.push(item),
            Err(error) => warnings.push(format!("{source_id}: {error}")),
        }
    }
    if items.is_empty() && (had_items || !warnings.is_empty()) {
        return Err(warnings.join("；"));
    }
    Ok(SearchResult { items, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn fixture(name: &str) -> Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/media/api")
            .join(format!("{name}.json"));
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn recorded_sources_keep_rights_identity_and_original_dimensions() {
        let poly = parse_polyhaven(&fixture("polyhaven"), "texture");
        assert_eq!(poly[0]["originalWidth"], 8192);
        assert_eq!(poly[0]["sourceId"], "aerial_asphalt_01");
        assert_eq!(poly[0]["use"], "page");
        let vam = parse_vam(&fixture("vam"));
        assert_eq!(vam[0]["sourceId"], "O72610");
        assert_eq!(vam[0]["use"], "reference");
        assert!(s(&vam[0]["iiifInfoUrl"]).ends_with("2006AU1797/info.json"));
        let wellcome = parse_wellcome(&fixture("wellcome"), "plate");
        assert_eq!(wellcome[0]["license"], "Public Domain Mark");
        assert_eq!(wellcome[0]["sourceId"], "avd5pgjb");
        let loc = parse_loc(&fixture("loc"), "photo");
        assert_eq!(loc[0]["license"], "Item-specific rights");
        assert_eq!(loc[0]["use"], "check");
        assert_eq!(parse_loc(&fixture("loc"), "blueprint").len(), 1);
        let nasa = parse_nasa(&fixture("nasa"), "science");
        assert_eq!(nasa[0]["sourceId"], "PIA00342");
        assert_eq!(nasa[0]["use"], "page");
        assert_eq!(
            parse_nasa(&fixture("nasa-video"), "video")[0]["video"],
            true
        );
        let commons = parse_commons(&fixture("commons"), "map");
        assert!(!commons.is_empty());
        assert!(number(&commons[0]["originalWidth"]) > 500);
        assert!(!s(&commons[0]["license"]).is_empty());
        let aic = parse_aic(&fixture("aic"));
        assert_eq!(aic[0]["originalWidth"], 12614);
        assert_eq!(aic[0]["originalHeight"], 9875);
        assert_eq!(aic[0]["use"], "page");
        let cleveland = parse_cleveland(&fixture("cleveland"));
        assert_eq!(cleveland[0]["sourceId"], "142748");
        assert!(number(&cleveland[0]["originalWidth"]) > 1000);
        let met = parse_met(&fixture("met"));
        assert_eq!(met[0]["sourceId"], "229");
        assert_eq!(met[0]["use"], "page");
        let japan = parse_japan(&fixture("japan"), "design");
        assert_eq!(japan[0]["sourceId"], "cobas-78821");
        assert!(s(&japan[0]["attribution"]).starts_with("出典：ColBase"));
        assert_eq!(japan[0]["license"], "CC BY 4.0");
        assert!(s(&japan[0]["imageUrl"]).contains("/slideshow/"));
        let rijks = parse_rijks(
            &json!({"object":fixture("rijks-object"),"digital":fixture("rijks-digital")}),
        );
        assert_eq!(rijks[0]["sourceId"], "200107779");
        assert!(s(&rijks[0]["iiifInfoUrl"]).ends_with("NXvPP/info.json"));
        assert_eq!(rijks[0]["use"], "page");
    }

    #[test]
    fn recorded_maps_choose_available_resolution_at_or_below_request() {
        let files = fixture("polyhaven-files");
        for keys in [&["Diffuse"][..], &["Rough"][..], &["nor_gl"][..]] {
            let selected = select_map(&files, keys, "1k").unwrap();
            assert_eq!(selected["resolution"], "1k");
            assert!(number(&selected["size"]) > 0);
            assert!(s(&selected["url"]).starts_with("https://"));
        }
        let sparse = json!({"Diffuse":{"2k":{"jpg":{"url":"https://example.test/2.jpg"}},"8k":{"jpg":{"url":"https://example.test/8.jpg"}}}});
        assert_eq!(
            select_map(&sparse, &["Diffuse"], "4k").unwrap()["resolution"],
            "2k"
        );
        assert_eq!(
            select_map(&sparse, &["Diffuse"], "1k").unwrap()["resolution"],
            "2k"
        );
    }

    #[test]
    fn urls_reject_credentials_and_upgrade_only_external_http() {
        assert!(normalize_url("file:///tmp/photo.jpg").is_err());
        assert!(normalize_url("https://user:pass@example.org/image.jpg").is_err());
        assert_eq!(
            normalize_url("//example.org/a.jpg").unwrap(),
            "https://example.org/a.jpg"
        );
        assert_eq!(
            normalize_url("http://example.org/a.jpg").unwrap(),
            "https://example.org/a.jpg"
        );
        assert_eq!(
            normalize_url("http://127.0.0.1:1234/a.jpg").unwrap(),
            "http://127.0.0.1:1234/a.jpg"
        );
    }

    #[test]
    fn aic_request_has_required_header_and_maps_recorded_response() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let address = format!("http://{}", server.server_addr());
        let worker = thread::spawn(move || {
            let request = server
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
            assert!(request.url().starts_with("/api/v1/artworks/search?"));
            assert!(request
                .headers()
                .iter()
                .any(|header| header.field.equiv("AIC-User-Agent")
                    && header.value.as_str() == USER_AGENT));
            request
                .respond(tiny_http::Response::from_string(fixture("aic").to_string()))
                .unwrap();
        });
        let endpoints = Endpoints {
            aic: address,
            ..Endpoints::default()
        };
        let result = search(
            &ureq::AgentBuilder::new().build(),
            &endpoints,
            &Task {
                provider: "aic",
                kind: "design",
                query: "chair".into(),
            },
            2,
        )
        .unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0]["originalWidth"], 12614);
        worker.join().unwrap();
    }

    #[test]
    fn following_dimensions_use_iiif_and_image_or_video_metadata() {
        let cases = [
            ("vam", "vam-info", "iiifInfoUrl"),
            ("wellcome", "wellcome-info", "iiifInfoUrl"),
            ("rijks", "rijks-info", "iiifInfoUrl"),
            ("nasa", "nasa-metadata", "manifestUrl"),
            ("nasa", "nasa-video-metadata", "manifestUrl"),
        ];
        for (provider, name, field) in cases {
            let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
            let address = format!("http://{}", server.server_addr());
            let info = fixture(name);
            let (width, height) = if name == "nasa-video-metadata" {
                (
                    number(&info["QuickTime:ImageWidth"]),
                    number(&info["QuickTime:ImageHeight"]),
                )
            } else if provider == "nasa" {
                (
                    number(&info["File:ImageWidth"]),
                    number(&info["File:ImageHeight"]),
                )
            } else {
                (number(&info["width"]), number(&info["height"]))
            };
            let worker = thread::spawn(move || {
                let request = server
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap()
                    .unwrap();
                assert!(request.url().ends_with(if field == "manifestUrl" {
                    "metadata.json"
                } else {
                    "info.json"
                }));
                request
                    .respond(tiny_http::Response::from_string(info.to_string()))
                    .unwrap();
            });
            let mut candidate = item(
                provider,
                "test".into(),
                "science",
                "test".into(),
                "".into(),
                "https://example.test/item".into(),
                "https://example.test/image.jpg".into(),
                "https://example.test/preview.jpg".into(),
                "CC0".into(),
                "".into(),
                "".into(),
            );
            candidate[field] = format!(
                "{address}/{}",
                if field == "manifestUrl" {
                    "collection.json"
                } else {
                    "info.json"
                }
            )
            .into();
            candidate["video"] = (name == "nasa-video-metadata").into();
            let actual = complete_item(
                &ureq::AgentBuilder::new().build(),
                &Endpoints::default(),
                candidate,
            )
            .unwrap();
            assert_eq!(
                (
                    number(&actual["originalWidth"]),
                    number(&actual["originalHeight"])
                ),
                (width, height),
                "{name}"
            );
            worker.join().unwrap();
        }
    }

    #[test]
    fn partial_image_requests_read_recorded_headers_and_reject_unknown_sizes() {
        for (provider, name, expected) in [
            ("met", "met-head", (2216, 1784)),
            ("japan", "japan-head", (328, 206)),
            ("loc", "met-head", (2216, 1784)),
        ] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/media/api")
                .join(format!("{name}.bin"));
            let bytes = std::fs::read(path).unwrap();
            let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
            let address = format!("http://{}/original.jpg", server.server_addr());
            let worker = thread::spawn(move || {
                let request = server
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap()
                    .unwrap();
                assert!(request
                    .headers()
                    .iter()
                    .any(|h| h.field.equiv("Range") && h.value.as_str() == "bytes=0-65535"));
                request
                    .respond(tiny_http::Response::from_data(bytes))
                    .unwrap();
            });
            let candidate = item(
                provider,
                "test".into(),
                "photo",
                "test".into(),
                "".into(),
                "https://example.test/item".into(),
                address.clone(),
                address,
                "CC0".into(),
                "".into(),
                "".into(),
            );
            let actual = complete_item(
                &ureq::AgentBuilder::new().build(),
                &Endpoints::default(),
                candidate,
            )
            .unwrap();
            assert_eq!(
                (
                    number(&actual["originalWidth"]),
                    number(&actual["originalHeight"])
                ),
                expected
            );
            worker.join().unwrap();
        }
        let mut data = fixture("aic");
        for record in data["data"].as_array_mut().unwrap() {
            record["thumbnail"] = Value::Null;
        }
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let address = format!("http://{}", server.server_addr());
        let worker = thread::spawn(move || {
            server
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap()
                .respond(tiny_http::Response::from_string(data.to_string()))
                .unwrap();
        });
        let result = search(
            &ureq::AgentBuilder::new().build(),
            &Endpoints {
                aic: address,
                ..Default::default()
            },
            &Task {
                provider: "aic",
                kind: "design",
                query: "chair".into(),
            },
            2,
        );
        assert!(result.err().unwrap().contains("无法取得原图尺寸"));
        worker.join().unwrap();
    }
}
