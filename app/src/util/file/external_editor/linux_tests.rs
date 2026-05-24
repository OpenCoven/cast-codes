use warp_util::path::LineAndColumnArg;

use super::{DesktopExecError, EditorMetadata};
use std::path::PathBuf;

#[cfg(test)]
fn with_files(tag: &str, contents: &str, cb: impl FnOnce(PathBuf, PathBuf) -> anyhow::Result<()>) {
    use crate::test_util::{Stub, VirtualFS};

    VirtualFS::test(tag, |dirs, mut sandbox| {
        sandbox.with_files(vec![
            Stub::FileWithContent("bar.desktop", contents),
            Stub::EmptyFile("foo.txt"),
        ]);

        let desktop_file_path = dirs.tests().join("bar.desktop");
        let content_file_path = dirs.tests().join("foo.txt");

        match cb(desktop_file_path, content_file_path) {
            Ok(_) => {}
            Err(err) => panic!("{err:?}"),
        };
    })
}

#[test]
fn test_missing_exec_command_errors() {
    with_files(
        "test_missing_exec_command_errors",
        "",
        |desktop, _content| {
            let result = EditorMetadata::try_new(desktop);

            assert!(matches!(result, Err(DesktopExecError::NoExec)));
            Ok(())
        },
    )
}

#[test]
fn test_exec_ending_on_percent_fails() {
    let data = r#"
    [Desktop Entry]
    Version=1.0
    Type=Application
    Exec=echo "hello world" %
    "#;
    with_files(
        "test_exec_ending_on_percent_fails",
        data,
        |desktop, content| {
            let metadata = EditorMetadata::try_new(desktop)?;
            let result = metadata.build_default_command(&content);
            assert!(matches!(result, Err(DesktopExecError::MalformedFieldCode)));
            Ok(())
        },
    )
}

#[test]
fn test_basic_exec_no_field_codes() {
    let data = r#"
    [Desktop Entry]
    Version=1.0
    Type=Application
    Exec=echo "hello world"
    "#;
    with_files(
        "test_basic_exec_no_field_codes",
        data,
        |desktop, content| {
            let metadata = EditorMetadata::try_new(desktop)?;
            let result = metadata.build_default_command(&content);
            assert!(result.is_ok());
            let cmd = result.unwrap();
            assert_eq!(cmd.get_program(), "sh");
            assert_eq!(
                cmd.get_args().collect::<Vec<_>>(),
                ["-c", "echo \"hello world\""]
            );
            Ok(())
        },
    )
}

#[test]
fn test_file_path_substitution() {
    let data = r#"
    [Desktop Entry]
    Version=1.0
    Type=Application
    Exec=cat %f
    "#;
    with_files("test_file_path_substitution", data, |desktop, content| {
        let metadata = EditorMetadata::try_new(desktop)?;
        let file_name = content.display().to_string();
        let quoted_file_name = EditorMetadata::shell_quote(&file_name);
        let result = metadata.build_default_command(&content);

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().get_args().collect::<Vec<_>>(),
            ["-c", format!("cat {quoted_file_name}").as_str()]
        );
        Ok(())
    });

    let data = r#"
    [Desktop Entry]
    Version=1.0
    Type=Application
    Exec=cat %F
    "#;
    with_files("test_file_path_substitution", data, |desktop, content| {
        let metadata = EditorMetadata::try_new(desktop)?;
        let file_name = content.display().to_string();
        let quoted_file_name = EditorMetadata::shell_quote(&file_name);
        let result = metadata.build_default_command(&content);

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().get_args().collect::<Vec<_>>(),
            ["-c", format!("cat {quoted_file_name}").as_str()]
        );
        Ok(())
    });
}

#[test]
fn test_double_quoted_file_path_substitution_uses_shell_owned_quotes() {
    let metadata = EditorMetadata {
        desktop_file_path: PathBuf::from("/tmp/editor.desktop"),
        exec: r#"cat "%f""#.to_string(),
        localized_name: None,
        icon: None,
    };
    let content = PathBuf::from("/tmp/$(touch pwn)' file.txt");
    let result = metadata.build_default_command(&content);

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().get_args().collect::<Vec<_>>(),
        ["-c", r#"cat '/tmp/$(touch pwn)'\'' file.txt'"#]
    );
}

#[test]
fn test_file_url_substitution() {
    let data = r#"
    [Desktop Entry]
    Version=1.0
    Type=Application
    Exec=open %u
    "#;
    with_files("test_file_url_substitution", data, |desktop, content| {
        let metadata = EditorMetadata::try_new(desktop)?;
        let file_name = content.display().to_string();
        let expected_file_uri = format!("file://{file_name}");
        let quoted_file_uri = EditorMetadata::shell_quote(&expected_file_uri);
        let result = metadata.build_default_command(&content);

        assert!(result.is_ok());

        assert_eq!(
            result.unwrap().get_args().collect::<Vec<_>>(),
            ["-c", &format!("open {quoted_file_uri}")]
        );
        Ok(())
    });

    let data = r#"
    [Desktop Entry]
    Version=1.0
    Type=Application
    Exec=open %U
    "#;
    with_files("test_file_url_substitution", data, |desktop, content| {
        let metadata = EditorMetadata::try_new(desktop)?;
        let file_name = content.display().to_string();
        let expected_file_uri = format!("file://{file_name}");
        let quoted_file_uri = EditorMetadata::shell_quote(&expected_file_uri);
        let result = metadata.build_default_command(&content);

        assert!(result.is_ok());

        assert_eq!(
            result.unwrap().get_args().collect::<Vec<_>>(),
            ["-c", &format!("open {quoted_file_uri}")]
        );
        Ok(())
    });
}

#[test]
fn test_remaining_substitutions() {
    let data = r#"
    [Desktop Entry]
    Version=1.0
    Type=Application
    Exec=echo %c && echo %i && echo %k && echo %%
    Name=Warp Test Application
    Icon=/foo/bar/icon.png
    "#;
    with_files("test_remaining_substitutions", data, |desktop, content| {
        let desktop_file_path = desktop.display().to_string();
        let metadata = EditorMetadata::try_new(desktop)?;
        let result = metadata.build_default_command(&content);

        assert!(result.is_ok());

        assert_eq!(
            result.unwrap().get_args().collect::<Vec<_>>(),
            ["-c", &format!("echo Warp Test Application && echo --icon /foo/bar/icon.png && echo {desktop_file_path} && echo %")]
        );
        Ok(())
    });
}

#[test]
fn test_jetbrains_command_no_line_numbers() {
    let data = r#"
    [Desktop Entry]
    Version=1.0
    Type=Application
    Exec=/snap/bin/phpstorm %f
    "#;

    with_files(
        "test_jetbrains_command_no_line_numbers",
        data,
        |desktop, content| {
            let metadata = EditorMetadata::try_new(desktop)?;
            let file_path = content.display().to_string();
            let quoted_file_path = EditorMetadata::shell_quote(&file_path);
            let result = metadata.build_jetbrains_command(&content, None);

            assert!(result.is_ok());

            assert_eq!(
                result.unwrap().get_args().collect::<Vec<_>>(),
                ["-c", &format!("/snap/bin/phpstorm {quoted_file_path}")]
            );
            Ok(())
        },
    );
}

#[test]
fn test_jetbrains_command_line_numbers() {
    let data = r#"
    [Desktop Entry]
    Version=1.0
    Type=Application
    Exec=/snap/bin/phpstorm %f
    "#;

    with_files(
        "test_jetbrains_command_line_numbers",
        data,
        |desktop, content| {
            let metadata = EditorMetadata::try_new(desktop)?;
            let file_path = content.display().to_string();
            let quoted_file_path = EditorMetadata::shell_quote(&file_path);
            let result = metadata.build_jetbrains_command(
                &content,
                Some(LineAndColumnArg {
                    line_num: 42,
                    column_num: None,
                }),
            );

            assert!(result.is_ok());

            assert_eq!(
                result.unwrap().get_args().collect::<Vec<_>>(),
                [
                    "-c",
                    &format!("/snap/bin/phpstorm --line 42 {quoted_file_path}")
                ]
            );
            Ok(())
        },
    );
}

#[test]
fn test_jetbrains_command_line_and_col_numbers() {
    let data = r#"
    [Desktop Entry]
    Version=1.0
    Type=Application
    Exec=/snap/bin/phpstorm %f
    "#;
    with_files(
        "test_jetbrains_command_line_and_col_numbers",
        data,
        |desktop, content| {
            let metadata = EditorMetadata::try_new(desktop)?;
            let file_path = content.display().to_string();
            let quoted_file_path = EditorMetadata::shell_quote(&file_path);
            let result = metadata.build_jetbrains_command(
                &content,
                Some(LineAndColumnArg {
                    line_num: 42,
                    column_num: Some(25),
                }),
            );

            assert!(result.is_ok());

            assert_eq!(
                result.unwrap().get_args().collect::<Vec<_>>(),
                [
                    "-c",
                    &format!("/snap/bin/phpstorm --line 42 --column 25 {quoted_file_path}")
                ]
            );
            Ok(())
        },
    );
}

#[test]
fn test_sublime_command_no_line_numbers() {
    let data = r#"
    [Desktop Entry]
    Version=1.0
    Type=Application
    Exec=/snap/bin/subl %f
    "#;
    with_files(
        "test_sublime_command_no_line_numbers",
        data,
        |desktop, content| {
            let metadata = EditorMetadata::try_new(desktop)?;
            let file_path = content.display().to_string();
            let quoted_file_path = EditorMetadata::shell_quote(&file_path);
            let result: Result<command::blocking::Command, DesktopExecError> =
                metadata.build_sublime_command(&content, None);

            assert!(result.is_ok());

            assert_eq!(
                result.unwrap().get_args().collect::<Vec<_>>(),
                ["-c", &format!("/snap/bin/subl {quoted_file_path}")]
            );
            Ok(())
        },
    );
}

#[test]
fn test_sublime_command_line_numbers() {
    let data = r#"
    [Desktop Entry]
    Version=1.0
    Type=Application
    Exec=/snap/bin/subl %f
    "#;
    with_files(
        "test_sublime_command_line_numbers",
        data,
        |desktop, content| {
            let metadata = EditorMetadata::try_new(desktop)?;
            let file_path = content.display().to_string();
            let quoted_file_path = EditorMetadata::shell_quote(&file_path);
            let result = metadata.build_sublime_command(
                &content,
                Some(LineAndColumnArg {
                    line_num: 42,
                    column_num: None,
                }),
            );

            assert!(result.is_ok());

            assert_eq!(
                result.unwrap().get_args().collect::<Vec<_>>(),
                ["-c", &format!("/snap/bin/subl {quoted_file_path}:42")]
            );
            Ok(())
        },
    );
}

#[test]
fn test_sublime_command_line_and_col_numbers() {
    let data = r#"
    [Desktop Entry]
    Version=1.0
    Type=Application
    Exec=/snap/bin/subl %f
    "#;
    with_files(
        "test_sublime_command_line_numbers",
        data,
        |desktop, content| {
            let metadata = EditorMetadata::try_new(desktop)?;
            let file_path = content.display().to_string();
            let quoted_file_path = EditorMetadata::shell_quote(&file_path);
            let result = metadata.build_sublime_command(
                &content,
                Some(LineAndColumnArg {
                    line_num: 42,
                    column_num: Some(25),
                }),
            );

            assert!(result.is_ok());

            assert_eq!(
                result.unwrap().get_args().collect::<Vec<_>>(),
                ["-c", &format!("/snap/bin/subl {quoted_file_path}:42:25")]
            );
            Ok(())
        },
    );
}
