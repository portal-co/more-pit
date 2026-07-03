package org.teavm.jso;

import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;

@Retention(RetentionPolicy.RUNTIME)
public @interface JSBody {
    String[] params() default {};
    String script() default "";
}
