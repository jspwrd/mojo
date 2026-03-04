pub fn default_std(lang: &str) -> &str {
    match lang {
        "c" => "c11",
        _ => "c++17",
    }
}

pub fn header_ext(lang: &str) -> &str {
    match lang {
        "c" => "h",
        _ => "hpp",
    }
}

pub fn main_file(lang: &str) -> (&str, &str) {
    match lang {
        "c" => (
            "c",
            "#include <stdio.h>\n\nint main(void) {\n    printf(\"Hello, world!\\n\");\n    return 0;\n}\n",
        ),
        _ => (
            "cpp",
            "#include <iostream>\n\nint main() {\n    std::cout << \"Hello, world!\" << std::endl;\n    return 0;\n}\n",
        ),
    }
}

pub fn lib_files(name: &str, lang: &str) -> (&'static str, String, String) {
    match lang {
        "c" => (
            "c",
            format!(
                "#ifndef {guard}_H\n#define {guard}_H\n\nint {name}_add(int a, int b);\n\n#endif\n",
                guard = name.to_uppercase(),
                name = name,
            ),
            format!(
                "#include \"{name}.h\"\n\nint {name}_add(int a, int b) {{\n    return a + b;\n}}\n",
                name = name,
            ),
        ),
        _ => (
            "cpp",
            format!(
                "#pragma once\n\nnamespace {name} {{\n\nint add(int a, int b);\n\n}} // namespace {name}\n",
                name = name,
            ),
            format!(
                "#include \"{name}.hpp\"\n\nnamespace {name} {{\n\nint add(int a, int b) {{\n    return a + b;\n}}\n\n}} // namespace {name}\n",
                name = name,
            ),
        ),
    }
}

pub fn test_file(name: &str, lang: &str) -> (&'static str, String) {
    match lang {
        "c" => (
            "c",
            format!(
                r#"#include <stdio.h>
#include <stdlib.h>

int main(void) {{
    /* test: basic assertion for {name} */
    if (1 + 1 != 2) {{
        fprintf(stderr, "FAIL: 1 + 1 != 2\n");
        return 1;
    }}
    printf("All tests passed.\n");
    return 0;
}}
"#,
                name = name,
            ),
        ),
        _ => (
            "cpp",
            format!(
                r#"#include <iostream>
#include <cassert>

int main() {{
    // test: basic assertion for {name}
    assert(1 + 1 == 2);
    std::cout << "All tests passed." << std::endl;
    return 0;
}}
"#,
                name = name,
            ),
        ),
    }
}
