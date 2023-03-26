use model::Prompt;
use tokio_postgres::Row;

pub(crate) fn from_row(row: Row) -> Prompt {
    Prompt {
        uid: row.get("uid"),
        field_key: row.get("field_key"),
        label: row.get("label"),
        kind: row.get("kind"),
        placeholder: row.get("placeholder"),
        required: row.get("required"),
        help_text: row.get("help_text"),
    }
}
