//! My Files index page.

use leptos::prelude::*;
use uf_product::components::{
    Body1, Caption1, Card, CardContent, ContentContainer, EmptyState, SpacingSize, Title3,
};
use uf_product::primitives::{
    Flex, Link, MessageBar, MessageBarIntent, Table, TableBody, TableCell, TableHeader,
    TableHeaderCell, TableRow,
};

use crate::components::FileStatusBadge;
use crate::server::{list_my_files, subscribe_list_my_files, MyFileRow};

/// My Files list across all File-trait tables for the signed-in user.
#[component]
pub fn MyFilesPage() -> impl IntoView {
    let ws_trigger = subscribe_list_my_files(|| {});
    let files_res = Resource::new(
        move || ws_trigger.get(),
        |_| async move { list_my_files().await },
    );

    view! {
        <ContentContainer data_testid="meson-my-files">
            <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                <Title3>"My Files"</Title3>
                <Body1>"Files you uploaded across apps that use the File trait."</Body1>

                <Suspense fallback=move || view! { <Card><CardContent>"Loading…"</CardContent></Card> }>
                    {move || match files_res.get() {
                        Some(Ok(files)) => {
                            let total = files.len();
                            view! {
                                <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                                    <Card>
                                        <CardContent>
                                            {if files.is_empty() {
                                                view! {
                                                    <EmptyState message="No files yet" />
                                                    <Caption1>"Uploads from apps that use the File trait appear here."</Caption1>
                                                }.into_any()
                                            } else {
                                                view! { <MyFilesTable files=files /> }.into_any()
                                            }}
                                        </CardContent>
                                    </Card>
                                    <Body1>"Showing " {total} " files"</Body1>
                                </Flex>
                            }.into_any()
                        }
                        Some(Err(e)) => view! {
                            <MessageBar intent=MessageBarIntent::Error>{e.to_string()}</MessageBar>
                        }.into_any(),
                        None => view! { <Card><CardContent>"Loading…"</CardContent></Card> }.into_any(),
                    }}
                </Suspense>
            </Flex>
        </ContentContainer>
    }
}

/// Renders one row per file with status badge, link, and source table.
#[component]
pub fn MyFilesTable(files: Vec<MyFileRow>) -> impl IntoView {
    view! {
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHeaderCell>"Name"</TableHeaderCell>
                    <TableHeaderCell>"Type"</TableHeaderCell>
                    <TableHeaderCell>"Size"</TableHeaderCell>
                    <TableHeaderCell>"Status"</TableHeaderCell>
                    <TableHeaderCell>"Uploaded"</TableHeaderCell>
                    <TableHeaderCell>"Source"</TableHeaderCell>
                </TableRow>
            </TableHeader>
            <TableBody>
                {files
                    .into_iter()
                    .map(|f| {
                        let href = format!("/meson/files/{}", urlencoding_path(&f.id));
                        let name = f.file_name;
                        let mime = f.mime_type;
                        let size = format_size(f.size_bytes);
                        let status = f.file_status;
                        let uploaded = f.uploaded_at;
                        let source = f.source_table;
                        view! {
                            <TableRow>
                                <TableCell>
                                    <Link href=href>{name}</Link>
                                </TableCell>
                                <TableCell>{mime}</TableCell>
                                <TableCell>{size}</TableCell>
                                <TableCell>
                                    <FileStatusBadge status=status />
                                </TableCell>
                                <TableCell>{uploaded}</TableCell>
                                <TableCell>{source}</TableCell>
                            </TableRow>
                        }
                    })
                    .collect_view()}
            </TableBody>
        </Table>
    }
}

#[allow(clippy::cast_precision_loss)]
fn format_size(bytes: i64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn urlencoding_path(id: &str) -> String {
    // Record ids use `table:id`; encode for path segment safety.
    id.replace(':', "%3A")
}
