import { open } from "@tauri-apps/plugin-dialog";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Plus } from "lucide-react";

import { Button } from "@/components/ui/button";
import { tauri, describeError } from "@/lib/tauri";

interface Props {
  onError?: (message: string) => void;
}

export function AddSiteButton({ onError }: Props) {
  const queryClient = useQueryClient();

  const addSite = useMutation({
    mutationFn: async () => {
      const picked = await open({
        directory: true,
        multiple: false,
        title: "Select a Hugo site folder",
      });
      if (!picked || typeof picked !== "string") return null;
      return tauri.workspaceAddSite(picked, null);
    },
    onSuccess: (added) => {
      if (added) queryClient.invalidateQueries({ queryKey: ["sites"] });
    },
    onError: (err) => onError?.(describeError(err)),
  });

  return (
    <Button onClick={() => addSite.mutate()} disabled={addSite.isPending}>
      <Plus className="size-4" />
      {addSite.isPending ? "Opening picker…" : "Add site"}
    </Button>
  );
}
