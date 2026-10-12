use crate::{
    matrix::{freeze, FrozenMatrix, Identity, State, Unit, Viewport},
    storage::{error, StoreError},
};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Definition {
    schema_version: u32,
    units: Vec<Unit>,
}

/// Read the unique top-level unit section; never infer units from code examples.
/// Portable IDs may have a separate display label. More complex matrices use the
/// explicit versioned code block instead of a second competing definition.
pub fn parse(markdown: &str, scope: Option<Vec<String>>) -> Result<FrozenMatrix, StoreError> {
    let text = markdown.replace("\r\n", "\n");
    let ranges = crate::target::section_ranges(&text)?;
    let (_, start, end) = ranges
        .iter()
        .find(|(name, _, _)| name == "单元")
        .ok_or_else(|| error(2, "Target needs a unique unit section"))?;
    let mut tables = Vec::new();
    let mut definitions = Vec::new();
    let mut table = None::<Vec<Vec<String>>>;
    let mut row = None::<Vec<String>>;
    let mut cell = None::<String>;
    let mut definition = None::<String>;
    let mut depth = 0usize;
    for event in Parser::new_ext(&text[*start..*end], Options::ENABLE_TABLES) {
        match event {
            Event::Start(tag) => {
                match tag {
                    Tag::Table(_) if depth == 0 => table = Some(Vec::new()),
                    Tag::TableHead | Tag::TableRow if table.is_some() => row = Some(Vec::new()),
                    Tag::TableCell if table.is_some() => cell = Some(String::new()),
                    Tag::CodeBlock(CodeBlockKind::Fenced(language))
                        if depth == 0 && language.trim() == "phoenix-ui-matrix" =>
                    {
                        definition = Some(String::new());
                    }
                    _ => (),
                }
                depth += 1;
            }
            Event::End(tag) => {
                match tag {
                    TagEnd::TableCell if cell.is_some() => {
                        row.as_mut()
                            .unwrap()
                            .push(cell.take().unwrap().trim().to_owned());
                    }
                    TagEnd::TableHead | TagEnd::TableRow if row.is_some() => {
                        table.as_mut().unwrap().push(row.take().unwrap());
                    }
                    TagEnd::Table if table.is_some() => tables.push(table.take().unwrap()),
                    TagEnd::CodeBlock if definition.is_some() => {
                        definitions.push(definition.take().unwrap());
                    }
                    _ => (),
                }
                depth -= 1;
            }
            Event::Text(value) | Event::Code(value) => {
                if let Some(cell) = &mut cell {
                    cell.push_str(&value);
                }
                if let Some(definition) = &mut definition {
                    definition.push_str(&value);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(cell) = &mut cell {
                    cell.push(' ');
                }
            }
            _ => (),
        }
    }
    if tables.len() + definitions.len() != 1 {
        return Err(error(
            2,
            "Target needs one unambiguous unit matrix definition",
        ));
    }
    let units = if let Some(definition) = definitions.pop() {
        let definition: Definition = serde_json::from_str(&definition).map_err(|e| error(2, e))?;
        if definition.schema_version != 1 {
            return Err(error(2, "Unsupported target matrix schema"));
        }
        definition.units
    } else {
        parse_table(tables.pop().unwrap())?
    };
    freeze(units, scope)
}

fn column(header: &[String], aliases: &[&str]) -> Result<usize, StoreError> {
    let indices = header
        .iter()
        .enumerate()
        .filter_map(|(index, name)| aliases.contains(&name.as_str()).then_some(index))
        .collect::<Vec<_>>();
    if indices.len() != 1 {
        return Err(error(
            2,
            format!("Unit table needs one {} column", aliases.join("/")),
        ));
    }
    Ok(indices[0])
}

fn identity(text: &str) -> Identity {
    let text = text.trim();
    let (id, label) = text.split_once(char::is_whitespace).unwrap_or((text, text));
    Identity {
        id: id.to_owned(),
        label: label.trim().to_owned(),
    }
}

fn items(text: &str) -> impl Iterator<Item = &str> {
    text.split(['、', ',']).map(str::trim)
}

fn viewport(text: &str) -> Result<Viewport, StoreError> {
    let words = text.split_whitespace().collect::<Vec<_>>();
    let (id, dimensions) = match words.as_slice() {
        [dimensions] => (None, *dimensions),
        [id, dimensions] => (Some(*id), *dimensions),
        _ => return Err(error(2, "Viewport needs width×height and optional ID/@DPR")),
    };
    let (dimensions, dpr) = dimensions.split_once('@').unwrap_or((dimensions, "1"));
    let dpr: f64 = dpr.parse().map_err(|e| error(2, e))?;
    let dimensions = dimensions.split(['×', 'x', 'X']).collect::<Vec<_>>();
    let [width, height] = dimensions.as_slice() else {
        return Err(error(2, "Viewport height must be explicit"));
    };
    let width: u32 = width.parse().map_err(|e| error(2, e))?;
    let height: u32 = height.parse().map_err(|e| error(2, e))?;
    Ok(Viewport {
        id: id.map(str::to_owned).unwrap_or_else(|| {
            format!("v{width}x{height}dpr{}", dpr.to_string().replace('.', "_"))
        }),
        width,
        height,
        dpr,
        full_page: false,
    })
}

fn parse_table(mut table: Vec<Vec<String>>) -> Result<Vec<Unit>, StoreError> {
    let header = table.remove(0);
    let unit = column(&header, &["单元"])?;
    let states = column(&header, &["状态", "必需状态"])?;
    let combined = header.iter().any(|name| name == "视口 × 主题");
    let views = column(
        &header,
        if combined {
            &["视口 × 主题"]
        } else {
            &["视口"]
        },
    )?;
    let themes = if combined {
        None
    } else {
        Some(column(&header, &["主题"])?)
    };
    table
        .into_iter()
        .map(|row| {
            if row.len() != header.len() {
                return Err(error(2, "Unit row does not match its table header"));
            }
            let (views, themes) = if let Some(themes) = themes {
                (row[views].as_str(), row[themes].as_str())
            } else {
                row[views].rsplit_once(" × ").ok_or_else(|| {
                    error(
                        2,
                        "Combined viewport/theme column needs an explicit × separator",
                    )
                })?
            };
            Ok(Unit {
                identity: identity(&row[unit]),
                states: items(&row[states])
                    .map(|state| State {
                        identity: identity(state),
                        steps: vec![identity("initial")],
                    })
                    .collect(),
                viewports: items(views).map(viewport).collect::<Result<_, _>>()?,
                themes: items(themes).map(identity).collect(),
            })
        })
        .collect()
}
