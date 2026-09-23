load("@rules_cc//cc:cc_library.bzl", "cc_library")
load("@rules_cc//cc:cc_test.bzl", "cc_test")

cc_test(
    name = "animated_object_tests",
    srcs = ["animated_object_tests.cc"],
    deps = [
        ":test_helpers",
        "@abseil-cpp//absl/strings:str_format",
        "@googletest//:gtest_main",
        "@oar",
        "@roar//src:roar",
    ],
)

cc_test(
    name = "downmix_tests",
    srcs = ["downmix_tests.cc"],
    deps = [
        ":test_helpers",
        "@abseil-cpp//absl/strings",
        "@abseil-cpp//absl/strings:str_format",
        "@googletest//:gtest_main",
        "@oar",
        "@roar//src:roar",
    ],
)

cc_test(
    name = "element_gains_tests",
    srcs = ["element_gains_tests.cc"],
    deps = [
        ":test_helpers",
        "@abseil-cpp//absl/strings:str_format",
        "@googletest//:gtest_main",
        "@oar",
        "@roar//src:roar",
    ],
)

cc_test(
    name = "error_conditions_tests",
    srcs = ["error_conditions_tests.cc"],
    deps = [
        ":test_helpers",
        "@abseil-cpp//absl/strings:str_format",
        "@googletest//:gtest_main",
        "@oar",
        "@roar//src:roar",
    ],
)

cc_test(
    name = "group_gains_tests",
    srcs = ["group_gains_tests.cc"],
    deps = [
        ":test_helpers",
        "@abseil-cpp//absl/strings:str_format",
        "@googletest//:gtest_main",
        "@oar",
        "@roar//src:roar",
    ],
)

cc_test(
    name = "head_tracking_tests",
    srcs = ["head_tracking_tests.cc"],
    deps = [
        ":test_helpers",
        "@abseil-cpp//absl/strings:str_format",
        "@googletest//:gtest_main",
        "@oar",
        "@roar//src:roar",
    ],
)

cc_test(
    name = "loudness_tests",
    srcs = ["loudness_tests.cc"],
    deps = [
        ":test_helpers",
        "@abseil-cpp//absl/strings:str_format",
        "@googletest//:gtest_main",
        "@oar",
        "@roar//src:roar",
    ],
)

cc_test(
    name = "simple_tests",
    srcs = ["simple_tests.cc"],
    deps = [
        ":test_helpers",
        "@abseil-cpp//absl/strings:str_format",
        "@googletest//:gtest_main",
        "@oar",
    ],
)

cc_library(
    name = "test_helpers",
    testonly = True,
    srcs = ["test_helpers.cc"],
    hdrs = [
        "test_helpers.h",
    ],
    visibility = ["@roar//equivalence_testing:__subpackages__"],
    deps = [
        "@abseil-cpp//absl/strings:str_format",
        "@oar",
        "@roar//src:roar",
    ],
)
