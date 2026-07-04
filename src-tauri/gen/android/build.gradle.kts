buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("com.android.tools.build:gradle:8.11.0")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25")
    }
}

allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

// rustls-platform-verifier — Maven repo for the Android component (trust-store access)
apply(from = "../../android-platform-verifier.gradle")

tasks.register("clean").configure {
    delete("build")
}

