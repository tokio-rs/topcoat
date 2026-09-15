fn main() {
    topcoat::icon::iconify::BuildConfig::new()
        .icon_set("lucide")
        .stage()
        .unwrap();

    topcoat::tailwind::BuildConfig::new()
        .input("app.css")
        .render()
        .unwrap();
}
