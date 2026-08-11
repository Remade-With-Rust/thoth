//! CSS `:root` sheet emitter for WebView / Dioxus injection.
//!
//! Enabled with the `css` feature. Requires `alloc`.

extern crate alloc;

use alloc::string::String;

use crate::tokens::{color, radius, space, type_scale};

/// Emit a single `:root { ... }` stylesheet using the default light token values.
///
/// Inject into a `<style>` tag or Dioxus document head. Apps override any
/// property afterward.
pub fn root_sheet() -> String {
    alloc::format!(
        concat!(
            ":root {{\n",
            "  {fg}: {fg_v};\n",
            "  {bg}: {bg_v};\n",
            "  {muted}: {muted_v};\n",
            "  {accent}: {accent_v};\n",
            "  {success}: {success_v};\n",
            "  {danger}: {danger_v};\n",
            "  {warn}: {warn_v};\n",
            "  {border}: {border_v};\n",
            "  {surface}: {surface_v};\n",
            "  {space_xs}: {space_xs_v};\n",
            "  {space_sm}: {space_sm_v};\n",
            "  {space_md}: {space_md_v};\n",
            "  {space_lg}: {space_lg_v};\n",
            "  {space_xl}: {space_xl_v};\n",
            "  {size_caption}: {size_caption_v};\n",
            "  {size_body}: {size_body_v};\n",
            "  {size_title}: {size_title_v};\n",
            "  {size_display}: {size_display_v};\n",
            "  {leading_body}: {leading_body_v};\n",
            "  {leading_tight}: {leading_tight_v};\n",
            "  {radius_sm}: {radius_sm_v};\n",
            "  {radius_md}: {radius_md_v};\n",
            "  {radius_lg}: {radius_lg_v};\n",
            "}}\n",
        ),
        fg = color::FG,
        fg_v = color::FG_VALUE,
        bg = color::BG,
        bg_v = color::BG_VALUE,
        muted = color::MUTED,
        muted_v = color::MUTED_VALUE,
        accent = color::ACCENT,
        accent_v = color::ACCENT_VALUE,
        success = color::SUCCESS,
        success_v = color::SUCCESS_VALUE,
        danger = color::DANGER,
        danger_v = color::DANGER_VALUE,
        warn = color::WARN,
        warn_v = color::WARN_VALUE,
        border = color::BORDER,
        border_v = color::BORDER_VALUE,
        surface = color::SURFACE,
        surface_v = color::SURFACE_VALUE,
        space_xs = space::XS,
        space_xs_v = space::XS_VALUE,
        space_sm = space::SM,
        space_sm_v = space::SM_VALUE,
        space_md = space::MD,
        space_md_v = space::MD_VALUE,
        space_lg = space::LG,
        space_lg_v = space::LG_VALUE,
        space_xl = space::XL,
        space_xl_v = space::XL_VALUE,
        size_caption = type_scale::SIZE_CAPTION,
        size_caption_v = type_scale::SIZE_CAPTION_VALUE,
        size_body = type_scale::SIZE_BODY,
        size_body_v = type_scale::SIZE_BODY_VALUE,
        size_title = type_scale::SIZE_TITLE,
        size_title_v = type_scale::SIZE_TITLE_VALUE,
        size_display = type_scale::SIZE_DISPLAY,
        size_display_v = type_scale::SIZE_DISPLAY_VALUE,
        leading_body = type_scale::LEADING_BODY,
        leading_body_v = type_scale::LEADING_BODY_VALUE,
        leading_tight = type_scale::LEADING_TIGHT,
        leading_tight_v = type_scale::LEADING_TIGHT_VALUE,
        radius_sm = radius::SM,
        radius_sm_v = radius::SM_VALUE,
        radius_md = radius::MD,
        radius_md_v = radius::MD_VALUE,
        radius_lg = radius::LG,
        radius_lg_v = radius::LG_VALUE,
    )
}
