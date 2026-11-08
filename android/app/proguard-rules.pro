# JNA e i binding UniFFI trovano classi e metodi per nome: non vanno toccati.
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-keep class uniffi.** { *; }
-dontwarn java.awt.**
-dontwarn com.sun.jna.**
-keep class com.journeyapps.barcodescanner.** { *; }
