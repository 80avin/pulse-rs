# Pulse: keep custom JNI bridge — methods called from native code
-keep class com.avinthakur080.pulse_rs.ShareBridge { *; }

# rustls-platform-verifier: JNI-based Android trust store verifier
-keep, includedescriptorclasses class org.rustls.platformverifier.** { *; }