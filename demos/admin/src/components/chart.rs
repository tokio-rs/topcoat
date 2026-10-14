use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, View, class, component, element_id, view},
};

/// Horizontal spacing between chart values, in SVG units.
const STEP: usize = 10;

/// Draws a filled line chart with values ordered from oldest to newest.
/// The vertical scale runs from the smallest value to the largest. The chart
/// fills its parent and uses its text color. Extra `attrs` go on the SVG.
#[component]
pub async fn line_chart(
    cx: &Cx,
    values: &[i64],
    #[default] mut attrs: Attributes,
) -> Result<impl View> {
    let fill_id = element_id(cx);
    let min = values.iter().copied().min().unwrap_or(0);
    let max = values.iter().copied().max().unwrap_or(0);
    let width = values.len().saturating_sub(1) * STEP;
    let points = values
        .iter()
        .enumerate()
        .map(|(index, &value)| {
            // Leave room for the stroke so the SVG does not clip it.
            let height = if max == min {
                40
            } else {
                10 + (value - min) * 80 / (max - min)
            };
            format!("{},{}", index * STEP, 100 - height)
        })
        .collect::<Vec<_>>()
        .join(" L");

    Ok(view! {
        <svg
            viewBox=(format!("0 0 {width} 100"))
            preserveAspectRatio="none"
            class=(class!("size-full", attrs.remove("class")))
            aria-hidden="true"
            (attrs)
        >
            <defs>
                <linearGradient id=(&fill_id) x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0" stop-color="currentColor" stop-opacity="0.3"></stop>
                    <stop offset="1" stop-color="currentColor" stop-opacity="0"></stop>
                </linearGradient>
            </defs>
            <path
                d=(format!("M0,100 L{points} L{width},100 Z"))
                fill=(format!("url(#{fill_id})"))
            ></path>
            <path
                d=(format!("M{points}"))
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linejoin="round"
                vector-effect="non-scaling-stroke"
            ></path>
        </svg>
    })
}

/// Draws bars from zero, ordered from oldest to newest, with the last bar
/// highlighted. The chart fills its parent and uses its text color.
/// Extra `attrs` go on the SVG.
#[component]
pub async fn bar_chart(values: &[i64], #[default] mut attrs: Attributes) -> Result<impl View> {
    let max = values.iter().copied().max().unwrap_or(0).max(1);
    let width = values.len() * STEP;

    Ok(view! {
        <svg
            viewBox=(format!("0 0 {width} 100"))
            preserveAspectRatio="none"
            class=(class!("size-full", attrs.remove("class")))
            aria-hidden="true"
            (attrs)
        >
            for (index, &value) in values.iter().enumerate() {
                let height = value * 90 / max;
                <rect
                    x=((index * STEP + 1).to_string())
                    y=((100 - height).to_string())
                    width=((STEP - 2).to_string())
                    height=(height.to_string())
                    fill="currentColor"
                    class=(if index + 1 == values.len() { "" } else { "opacity-40" })
                ></rect>
            }
        </svg>
    })
}
