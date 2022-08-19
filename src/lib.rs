use std::{collections::HashMap, fmt::Formatter, result};

#[derive(Debug)]
pub enum ParseError {
    MissingOpenBrace(usize),
    MissingCloseBrace(usize),
    MissingTemplate(String),
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTemplate(name) => write!(
                f,
                "sub_templates does not have any template indexed under: {}",
                name
            ),
            Self::MissingCloseBrace(loc) => write!(
                f,
                "Open brace at {} does not have a corresponding close brace",
                loc
            ),
            Self::MissingOpenBrace(loc) => write!(
                f,
                "Close brace at {} does not have a corresponding open brace",
                loc
            ),
        }
    }
}

pub struct NestedTemplate {
    body: String,
    sub_templates: HashMap<String, NestedTemplate>,
}

fn render_helper(body: &str) -> Result<Vec<(bool, String)>, ParseError> {
    // Handle any escaped opening braces
    if let Some(start_escape) = body.find("{{") {
        let pre_escape = &body[..start_escape];

        // This checks if there is anything following the escaped brace
        let mut post_escape = "";
        if start_escape + 2 < body.len() {
            post_escape = &body[start_escape + 2..];
        }

        // Render everything before the escaped brace
        let mut pre_vec = render_helper(pre_escape)?;
        pre_vec.push((false, "{".to_string()));

        // Render everything after the escaped brace
        let mut post_vec = render_helper(post_escape)?;
        pre_vec.append(&mut post_vec);

        return Ok(pre_vec);
    }

    // Handle any escaped closing braces
    if let Some(start_escape) = body.find("}}") {
        let pre_escape = &body[..start_escape];

        // This checks if there is anything following the escaped brace
        let mut post_escape = "";
        if start_escape + 2 < body.len() {
            post_escape = &body[start_escape + 2..];
        }

        // Render everything before the escaped brace
        let mut pre_vec = render_helper(pre_escape)?;
        pre_vec.push((false, "}".to_string()));

        let mut post_vec = render_helper(post_escape)?;
        pre_vec.append(&mut post_vec);

        return Ok(pre_vec);
    }

    let start_template = body.find("{");
    let end_template = body.find("}");

    if start_template.is_none() && end_template.is_none() {
        // There are no template strings left, so return the whole string

        return Ok(vec![(false, body.to_string())]);
    } else if start_template.is_some() && end_template.is_none() {
        // The template is never closed, so return a missing close brace error

        return Err(ParseError::MissingCloseBrace(start_template.unwrap()));
    } else if start_template.is_none() && end_template.is_some() {
        // The template is never opened, so return a missing open brace error

        return Err(ParseError::MissingOpenBrace(end_template.unwrap()));
    }

    // Check to make sure that opening brace comes before the closing brace. Otherwise, treat it as
    // a missing open brace error
    if start_template.unwrap() > end_template.unwrap() {
        return Err(ParseError::MissingOpenBrace(end_template.unwrap()));
    }

    let pre_template = &body[..start_template.unwrap()]; // String preceding template start
    let post_template = &body[end_template.unwrap() + 1..]; // String proceding end of template
    let mut post_vec = render_helper(post_template)?; // Render everything after template
    let template_name = &body[start_template.unwrap() + 1..end_template.unwrap()].trim();

    // Everything before the template has already been rendered, so just return the string. The
    let mut result = vec![
        (false, pre_template.to_string()),
        (true, template_name.to_string()),
    ];
    result.append(&mut post_vec);

    Ok(result)
}

impl NestedTemplate {
    pub fn new(body: &str) -> NestedTemplate {
        NestedTemplate {
            body: body.to_string(),
            sub_templates: HashMap::new(),
        }
    }

    pub fn add_sub_template(&mut self, name: &str, template: NestedTemplate) {
        self.sub_templates.insert(name.to_string(), template);
    }

    pub fn render(&self) -> Result<String, ParseError> {
        let pairs = render_helper(&self.body)?;
        let mut rendered_template = String::new();

        for (is_template, value) in pairs.iter() {
            if *is_template {
                let sub_template = match self.sub_templates.get(value) {
                    Some(t) => t,
                    None => return Err(ParseError::MissingTemplate(value.to_string())),
                };

                rendered_template.push_str(&sub_template.render()?);
            } else {
                rendered_template.push_str(value);
            }
        }

        Ok(rendered_template)
    }
}

#[cfg(test)]
mod NestedTemplate_tests {
    use super::*;

    #[test]
    fn test_render() {
        let mut parent = NestedTemplate::new("<!DOCTYPE html><body>{first_child}</body>");
        let mut first_child = NestedTemplate::new("<div>This is a test</div><script>{second_child}</script>");
        let second_child = NestedTemplate::new("second_child");

        first_child.add_sub_template("second_child", second_child);
        parent.add_sub_template("first_child", first_child);
        assert_eq!(parent.render().unwrap(), "<!DOCTYPE html><body><div>This is a test</div><script>second_child</script></body>");
    }
}

#[cfg(test)]
mod render_helper_tests {
    use super::*;

    #[test]
    fn test_render_helper_missing_open() {
        match render_helper("something }") {
            Ok(_) => panic!("render_helper did not catch missing \"{{\""),
            Err(ParseError::MissingOpenBrace(_)) => (),
            _ => panic!("render_helper caught wrong error"),
        }
    }

    #[test]
    fn test_render_helper_missing_close() {
        match render_helper("something {") {
            Ok(_) => panic!("render_helper did not catch missing \"}}\""),
            Err(ParseError::MissingCloseBrace(_)) => (),
            _ => panic!("render_helper caught wrong error"),
        }
    }

    #[test]
    fn test_render_helper_success() {
        let val = render_helper("This is a {successful} test of the {helper_function}").unwrap();
        assert_eq!(
            val,
            vec![
                (false, "This is a ".to_string()),
                (true, "successful".to_string()),
                (false, " test of the ".to_string()),
                (true, "helper_function".to_string()),
                (false, String::new())
            ]
        );
    }

    #[test]
    fn test_template_inside_template() {
        match render_helper("{ {something} }") {
            Ok(_) => panic!("render_helper allowed template inside another template"),
            _ => (),
        }
    }

    #[test]
    fn test_empty_template_with_helper() {
        assert_eq!(
            render_helper("{}").unwrap(),
            vec![
                (false, String::new()),
                (true, String::new()),
                (false, String::new())
            ]
        );
    }

    #[test]
    fn test_escape_with_for_helper() {
        assert_eq!(
            render_helper("this should {{ be escaped }}").unwrap(),
            vec![
                (false, "this should ".to_string()),
                (false, "{".to_string()),
                (false, " be escaped ".to_string()),
                (false, "}".to_string()),
                (false, String::new())
            ]
        );
    }

    #[test]
    fn test_render_helper() {
        let template_str = "{ template }{other_template} not template {{}}";
        assert_eq!(
            render_helper(template_str).unwrap(),
            vec![
                (false, String::new()),
                (true, "template".to_string()),
                (false, String::new()),
                (true, "other_template".to_string()),
                (false, " not template ".to_string()),
                (false, "{".to_string()),
                (false, String::new()),
                (false, "}".to_string()),
                (false, String::new()),
            ]
        );
    }
}
