import path from "node:path";

export function cargoTargetDirectory(metadataJson) {
  let metadata;
  try {
    metadata = JSON.parse(metadataJson);
  } catch (error) {
    throw new Error("Cargo returned invalid metadata JSON.", { cause: error });
  }

  if (
    typeof metadata.target_directory !== "string" ||
    !path.isAbsolute(metadata.target_directory)
  ) {
    throw new Error(
      "Cargo metadata did not contain an absolute target directory.",
    );
  }

  return metadata.target_directory;
}
