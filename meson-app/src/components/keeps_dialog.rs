//! "What My Files keeps" dialog, opened from a link under the page title.

use leptos::prelude::*;
use uf_product::components::{Body1, Caption1};
use uf_product::primitives::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Flex, List, ListItem,
};

/// Link plus dialog. The open state is local, so opening never touches the
/// page's resources.
#[component]
pub fn KeepsDialog() -> impl IntoView {
    let open = RwSignal::new(false);
    view! {
        <Button
            appearance=ButtonAppearance::Transparent
            attr:data-testid="meson-keeps-link"
            on_click=Callback::new(move |_| open.set(true))
        >
            "What My Files keeps"
        </Button>
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"What My Files keeps"</DialogTitle>
                    <DialogContent>
                        <div data-testid="meson-keeps-dialog">
                            <Flex vertical=true gap=uf_product::components::SpacingSize::Size160.flex_gap()>
                                <Body1>"For every file you upload in any app on this site, Meson keeps:"</Body1>
                                <List>
                                    <ListItem>"The file's name, type and size, and when you uploaded it"</ListItem>
                                    <ListItem>"Which app stored it, and where in storage"</ListItem>
                                    <ListItem>"The virus scan result"</ListItem>
                                    <ListItem>"That you uploaded it"</ListItem>
                                </List>
                                <Body1>
                                    "My Files reads these to list your files and show a preview. Only you can see them here; the app that stored a file decides who else can see it there."
                                </Body1>
                                <Body1>
                                    "The daily storage run counts your files and their sizes, by app. Site owners see that total next to your name; they don't see the file names."
                                </Body1>
                                <Caption1>
                                    "Delete a file and all of this goes with it. Close your account and Meson deletes your files within 30 days."
                                </Caption1>
                            </Flex>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            attr:data-testid="meson-keeps-close"
                            on_click=Callback::new(move |_| open.set(false))
                        >
                            "Close"
                        </Button>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}
