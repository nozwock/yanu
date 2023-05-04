use std::fmt;

use console::style;
use time::{macros::format_description, OffsetDateTime};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    fmt::{format, FmtContext, FormatEvent, FormatFields, FormattedFields},
    registry::LookupSpan,
};

pub struct CustomFmt;

impl<S, N> FormatEvent<S, N> for CustomFmt
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        // Create timestamp
        // https://time-rs.github.io/book/api/well-known-format-descriptions.html#rfc-2822
        let time_fmt = format_description!(
            "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second]"
        );
        let time_now = OffsetDateTime::now_utc()
            .format(time_fmt)
            .expect("Format should be valid");

        let metadata = event.metadata();
        let message = format!(
            "{} {} {}",
            style(time_now).dim(),
            match *metadata.level() {
                Level::TRACE => style(metadata.level()).magenta(),
                Level::DEBUG => style(metadata.level()).blue(),
                Level::INFO => style(metadata.level()).green(),
                Level::WARN => style(metadata.level()).yellow(),
                Level::ERROR => style(metadata.level()).red(),
            },
            style(format!(
                "[{}:{}]",
                metadata.file().unwrap_or_default(),
                metadata.line().unwrap_or_default()
            ))
            .cyan()
        );
        write!(&mut writer, "{}: ", message)?;

        // No idea what the following does, copy pasted from docs.

        // Format all the spans in the event's span context.
        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                write!(writer, "{}", span.name())?;

                // `FormattedFields` is a formatted representation of the span's
                // fields, which is stored in its extensions by the `fmt` layer's
                // `new_span` method. The fields will have been formatted
                // by the same field formatter that's provided to the event
                // formatter in the `FmtContext`.
                let ext = span.extensions();
                let fields = &ext
                    .get::<FormattedFields<N>>()
                    .expect("will never be `None`");

                // Skip formatting the fields if the span had no fields.
                if !fields.is_empty() {
                    write!(writer, "{{{}}}", fields)?;
                }
                write!(writer, ": ")?;
            }
        }

        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}
