export interface AppSettings {
    theme: "dark" | "light" | "system";
    language: string;
    currency: "EUR" | "GBP" | "USD";
    default_match_mode: "live" | "spectator" | "delegate";
    auto_save: boolean;
    match_speed: "slow" | "normal" | "fast";
    show_match_commentary: boolean;
    confirm_advance: boolean;
    ui_scale: "small" | "normal" | "large" | "xlarge";
    high_contrast: boolean;
}