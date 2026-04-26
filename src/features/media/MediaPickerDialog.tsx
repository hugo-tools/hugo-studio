import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { type AssetRef, type Site } from "@/lib/tauri";
import { MediaLibrary } from "./MediaLibrary";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  site: Site;
  bundleContentId?: string | null;
  bundleLabel?: string | null;
  /** Called when the user picks a media item — typically the editor
   *  closes the dialog and inserts a markdown link at the caret. */
  onSelect: (asset: AssetRef) => void;
}

export function MediaPickerDialog({
  open,
  onOpenChange,
  site,
  bundleContentId,
  bundleLabel,
  onSelect,
}: Props) {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent className="flex h-[80vh] max-h-[760px] w-[min(960px,90vw)] max-w-none flex-col gap-0 overflow-hidden p-0 sm:max-w-none">
        <AlertDialogHeader className="border-b px-4 py-3">
          <AlertDialogTitle className="text-sm">Insert media</AlertDialogTitle>
        </AlertDialogHeader>
        <div className="flex-1 overflow-hidden">
          <MediaLibrary
            site={site}
            bundleContentId={bundleContentId}
            bundleLabel={bundleLabel}
            initialScope={bundleContentId ? "bundle" : "static"}
            dropPriority={10}
            onSelect={(a) => {
              onSelect(a);
              onOpenChange(false);
            }}
          />
        </div>
        <AlertDialogFooter className="border-t bg-muted/20 px-4 py-2">
          <AlertDialogCancel className="m-0">Close</AlertDialogCancel>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
