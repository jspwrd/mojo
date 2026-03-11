use crate::cli::Framework;

/// Extra build configuration generated for a framework.
pub struct FrameworkConfig {
    /// Extra lines to append inside `[build]` (cflags, ldflags, libs).
    pub build_toml: &'static str,
    /// Extra content to append after the `[dependencies]` section.
    pub extra_toml: &'static str,
    /// Overrides the default main source file content.
    pub main_content: &'static str,
    /// Source file extension override (e.g. "cpp"). Empty means use default.
    pub src_ext: &'static str,
    /// Additional files to create: (relative path, content).
    pub extra_files: &'static [(&'static str, &'static str)],
    /// Hint printed after project creation.
    pub hint: &'static str,
    /// Force the language to this value, or empty to keep the user's choice.
    pub force_lang: &'static str,
    /// Force the std to this value, or empty to keep the default.
    pub force_std: &'static str,
}

pub fn framework_config(fw: Framework) -> FrameworkConfig {
    match fw {
        Framework::Qt => qt_config(),
        Framework::Gtk => gtk_config(),
        Framework::Libcurl => libcurl_config(),
        Framework::Grpc => grpc_config(),
        Framework::Gtest => gtest_config(),
        Framework::Boost => boost_config(),
        Framework::Freertos => freertos_config(),
        Framework::Zephyr => zephyr_config(),
    }
}

fn qt_config() -> FrameworkConfig {
    FrameworkConfig {
        build_toml: r#"cflags = ["-fPIC"]
libs = ["Qt5Widgets", "Qt5Core", "Qt5Gui"]
"#,
        extra_toml: r#"
[scripts]
pre_build = "moc include/mainwindow.hpp -o src/moc_mainwindow.cpp 2>/dev/null || true"
"#,
        main_content: r#"#include <QApplication>
#include <QMainWindow>
#include <QLabel>

int main(int argc, char *argv[]) {
    QApplication app(argc, argv);

    QMainWindow window;
    window.setWindowTitle("Mojo Qt App");
    window.resize(400, 300);

    QLabel *label = new QLabel("Hello from Mojo + Qt!", &window);
    label->setAlignment(Qt::AlignCenter);
    window.setCentralWidget(label);

    window.show();
    return app.exec();
}
"#,
        src_ext: "cpp",
        extra_files: &[(
            "include/mainwindow.hpp",
            r#"#pragma once

#include <QMainWindow>

class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow() override = default;
};
"#,
        )],
        hint: "Install Qt5 dev packages (e.g. apt install qtbase5-dev) to build.",
        force_lang: "c++",
        force_std: "c++17",
    }
}

fn gtk_config() -> FrameworkConfig {
    FrameworkConfig {
        build_toml: r#"cflags = ["`pkg-config --cflags gtk+-3.0`"]
ldflags = ["`pkg-config --libs gtk+-3.0`"]
"#,
        extra_toml: "",
        main_content: r#"#include <gtk/gtk.h>

static void on_activate(GtkApplication *app, gpointer user_data) {
    (void)user_data;
    GtkWidget *window = gtk_application_window_new(app);
    gtk_window_set_title(GTK_WINDOW(window), "Mojo GTK App");
    gtk_window_set_default_size(GTK_WINDOW(window), 400, 300);

    GtkWidget *label = gtk_label_new("Hello from Mojo + GTK!");
    gtk_container_add(GTK_CONTAINER(window), label);

    gtk_widget_show_all(window);
}

int main(int argc, char *argv[]) {
    GtkApplication *app = gtk_application_new("com.mojo.example", G_APPLICATION_DEFAULT_FLAGS);
    g_signal_connect(app, "activate", G_CALLBACK(on_activate), NULL);
    int status = g_application_run(G_APPLICATION(app), argc, argv);
    g_object_unref(app);
    return status;
}
"#,
        src_ext: "c",
        extra_files: &[],
        hint: "Install GTK3 dev packages (e.g. apt install libgtk-3-dev) to build.",
        force_lang: "c",
        force_std: "c11",
    }
}

fn libcurl_config() -> FrameworkConfig {
    FrameworkConfig {
        build_toml: r#"libs = ["curl"]
"#,
        extra_toml: "",
        main_content: r#"#include <stdio.h>
#include <curl/curl.h>

int main(void) {
    CURL *curl = curl_easy_init();
    if (!curl) {
        fprintf(stderr, "Failed to initialize libcurl\n");
        return 1;
    }

    curl_easy_setopt(curl, CURLOPT_URL, "https://httpbin.org/get");
    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);

    printf("Fetching https://httpbin.org/get ...\n");
    CURLcode res = curl_easy_perform(curl);
    if (res != CURLE_OK) {
        fprintf(stderr, "curl error: %s\n", curl_easy_strerror(res));
    }

    curl_easy_cleanup(curl);
    return (int)res;
}
"#,
        src_ext: "c",
        extra_files: &[],
        hint: "Install libcurl dev packages (e.g. apt install libcurl4-openssl-dev) to build.",
        force_lang: "c",
        force_std: "c11",
    }
}

fn grpc_config() -> FrameworkConfig {
    FrameworkConfig {
        build_toml: r#"cflags = ["`pkg-config --cflags grpc++ protobuf`"]
ldflags = ["`pkg-config --libs grpc++ protobuf`"]
libs = ["grpc++", "protobuf"]
"#,
        extra_toml: r#"
[scripts]
pre_build = "protoc --cpp_out=src --grpc_out=src --plugin=protoc-gen-grpc=`which grpc_cpp_plugin` proto/hello.proto 2>/dev/null || true"
"#,
        main_content: r#"#include <iostream>
#include <grpcpp/grpcpp.h>

int main() {
    std::cout << "gRPC version: " << grpc::Version() << std::endl;
    std::cout << "Mojo + gRPC project ready." << std::endl;
    std::cout << "Define your .proto files in proto/ and rebuild." << std::endl;
    return 0;
}
"#,
        src_ext: "cpp",
        extra_files: &[(
            "proto/hello.proto",
            r#"syntax = "proto3";

package hello;

service Greeter {
    rpc SayHello (HelloRequest) returns (HelloReply);
}

message HelloRequest {
    string name = 1;
}

message HelloReply {
    string message = 1;
}
"#,
        )],
        hint: "Install gRPC dev packages (e.g. apt install libgrpc++-dev protobuf-compiler-grpc) to build.",
        force_lang: "c++",
        force_std: "c++17",
    }
}

fn gtest_config() -> FrameworkConfig {
    FrameworkConfig {
        build_toml: r#"libs = ["gtest", "gtest_main", "pthread"]
"#,
        extra_toml: "",
        main_content: r#"#include <gtest/gtest.h>

int add(int a, int b) {
    return a + b;
}

TEST(MathTest, Addition) {
    EXPECT_EQ(add(2, 3), 5);
    EXPECT_EQ(add(-1, 1), 0);
    EXPECT_EQ(add(0, 0), 0);
}

TEST(MathTest, Negative) {
    EXPECT_EQ(add(-2, -3), -5);
}

int main(int argc, char **argv) {
    ::testing::InitGoogleTest(&argc, argv);
    return RUN_ALL_TESTS();
}
"#,
        src_ext: "cpp",
        extra_files: &[],
        hint: "Install Google Test dev packages (e.g. apt install libgtest-dev) to build.",
        force_lang: "c++",
        force_std: "c++17",
    }
}

fn boost_config() -> FrameworkConfig {
    FrameworkConfig {
        build_toml: r#"libs = ["boost_filesystem", "boost_system"]
"#,
        extra_toml: "",
        main_content: r#"#include <iostream>
#include <boost/filesystem.hpp>
#include <boost/version.hpp>

namespace fs = boost::filesystem;

int main() {
    std::cout << "Boost version: " << BOOST_LIB_VERSION << std::endl;
    std::cout << "Current path: " << fs::current_path() << std::endl;

    std::cout << "\nDirectory contents:" << std::endl;
    for (const auto &entry : fs::directory_iterator(fs::current_path())) {
        std::cout << "  " << entry.path().filename() << std::endl;
    }

    return 0;
}
"#,
        src_ext: "cpp",
        extra_files: &[],
        hint: "Install Boost dev packages (e.g. apt install libboost-all-dev) to build.",
        force_lang: "c++",
        force_std: "c++17",
    }
}

fn freertos_config() -> FrameworkConfig {
    FrameworkConfig {
        build_toml: r#"cflags = ["-DFREERTOS", "-Ideps/FreeRTOS-Kernel/include"]
"#,
        extra_toml: r#"
[dependencies]
FreeRTOS-Kernel = { git = "https://github.com/FreeRTOS/FreeRTOS-Kernel.git", tag = "V11.1.0" }
"#,
        main_content: r#"#include <stdio.h>

/* FreeRTOS headers */
#include "FreeRTOS.h"
#include "task.h"

void vTaskBlink(void *pvParameters) {
    (void)pvParameters;
    for (;;) {
        printf("LED toggle\n");
        /* vTaskDelay(pdMS_TO_TICKS(500)); */
    }
}

int main(void) {
    printf("Mojo + FreeRTOS project ready.\n");
    printf("Configure FreeRTOSConfig.h for your target board.\n");

    /*
     * xTaskCreate(vTaskBlink, "Blink", configMINIMAL_STACK_SIZE, NULL, 1, NULL);
     * vTaskStartScheduler();
     */

    return 0;
}
"#,
        src_ext: "c",
        extra_files: &[(
            "include/FreeRTOSConfig.h",
            r#"#ifndef FREERTOS_CONFIG_H
#define FREERTOS_CONFIG_H

/* Minimal FreeRTOS configuration — customize for your target. */
#define configUSE_PREEMPTION            1
#define configUSE_IDLE_HOOK             0
#define configUSE_TICK_HOOK             0
#define configCPU_CLOCK_HZ              ((unsigned long)72000000)
#define configTICK_RATE_HZ              ((TickType_t)1000)
#define configMAX_PRIORITIES            5
#define configMINIMAL_STACK_SIZE        ((unsigned short)128)
#define configTOTAL_HEAP_SIZE           ((size_t)(16 * 1024))
#define configMAX_TASK_NAME_LEN         16
#define configUSE_16_BIT_TICKS          0
#define configIDLE_SHOULD_YIELD         1

/* Memory allocation. */
#define configSUPPORT_STATIC_ALLOCATION  0
#define configSUPPORT_DYNAMIC_ALLOCATION 1

#endif /* FREERTOS_CONFIG_H */
"#,
        )],
        hint: "FreeRTOS Kernel will be fetched as a git dependency. Configure FreeRTOSConfig.h for your board.",
        force_lang: "c",
        force_std: "c11",
    }
}

fn zephyr_config() -> FrameworkConfig {
    FrameworkConfig {
        build_toml: r#"cflags = ["-DZEPHYR"]
"#,
        extra_toml: "",
        main_content: r#"#include <stdio.h>

/*
 * Zephyr RTOS project skeleton.
 *
 * Zephyr uses its own CMake-based build system (west).
 * This Mojo project provides the source structure;
 * use `west build` for the actual firmware build.
 */

int main(void) {
    printf("Mojo + Zephyr project ready.\n");
    printf("Use `west build` to compile for your target board.\n");
    return 0;
}
"#,
        src_ext: "c",
        extra_files: &[
            (
                "prj.conf",
                r#"# Zephyr project configuration
CONFIG_PRINTK=y
CONFIG_LOG=y
"#,
            ),
            (
                "CMakeLists.txt",
                r#"# Zephyr requires its own CMake build — this file is used by `west build`.
cmake_minimum_required(VERSION 3.20.0)
find_package(Zephyr REQUIRED HINTS $ENV{ZEPHYR_BASE})
project(app)
target_sources(app PRIVATE src/main.c)
"#,
            ),
        ],
        hint: "Install the Zephyr SDK and west tool. Use `west build` to compile for your target board.",
        force_lang: "c",
        force_std: "c11",
    }
}
