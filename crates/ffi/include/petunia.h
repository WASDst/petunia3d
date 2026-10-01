/**
 * @file petunia.h
 * @brief C-ABI / FFI Interface for Petunia3D External Frontends (Gauntlet G10).
 *
 * This header enables building external user interfaces for Petunia3D
 * in C, C++, C#, Python, Go, and any language capable of consuming C-ABI libraries.
 */

#ifndef PETUNIA_H
#define PETUNIA_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ------------------------------------------------------------------------- */
/* Error Codes                                                               */
/* ------------------------------------------------------------------------- */

#define PETUNIA_OK                     0
#define PETUNIA_ERR_NULL_PTR         -1
#define PETUNIA_ERR_INVALID_UTF8     -2
#define PETUNIA_ERR_OPERATION_FAILED -3
#define PETUNIA_ERR_BUFFER_TOO_SMALL -4
#define PETUNIA_ERR_NOT_FOUND        -5

/* ------------------------------------------------------------------------- */
/* Opaque Types                                                              */
/* ------------------------------------------------------------------------- */

/**
 * @brief Opaque handle to a Petunia3D engine context and project session.
 */
typedef struct PetuniaContext PetuniaContext;

/* ------------------------------------------------------------------------- */
/* Context Lifecycle                                                         */
/* ------------------------------------------------------------------------- */

/**
 * @brief Create a new Petunia3D session context initialized with a default project.
 * @param lang Preferred language code (e.g. "en", "pt-BR"), or NULL for "en".
 * @return Pointer to opaque PetuniaContext, or NULL on allocation failure.
 */
PetuniaContext* petunia_context_create(const char* lang);

/**
 * @brief Destroy a Petunia3D session context and free all associated memory.
 * @param ctx Pointer to context. Safe to pass NULL.
 */
void petunia_context_destroy(PetuniaContext* ctx);

/* ------------------------------------------------------------------------- */
/* Project & File I/O Operations                                             */
/* ------------------------------------------------------------------------- */

/**
 * @brief Reset the session with a new empty project containing a default cube.
 */
int32_t petunia_new_project(PetuniaContext* ctx);

/**
 * @brief Load a project from a .petunia file path.
 */
int32_t petunia_load_project(PetuniaContext* ctx, const char* path);

/**
 * @brief Save current project to a .petunia file path.
 */
int32_t petunia_save_project(PetuniaContext* ctx, const char* path);

/**
 * @brief Import a Wavefront OBJ file as an asset in the current project.
 */
int32_t petunia_import_obj(PetuniaContext* ctx, const char* path);

/**
 * @brief Import a glTF/GLB file with materials and textures as assets.
 */
int32_t petunia_import_gltf(PetuniaContext* ctx, const char* path);

/**
 * @brief Export the active asset to Wavefront OBJ format.
 */
int32_t petunia_export_obj(PetuniaContext* ctx, const char* path);

/**
 * @brief Export all assets in the project to binary GLB (glTF 2.0) format.
 */
int32_t petunia_export_glb(PetuniaContext* ctx, const char* path);

/* ------------------------------------------------------------------------- */
/* Modeling Primitives & Commands                                            */
/* ------------------------------------------------------------------------- */

/**
 * @brief Add a canonical primitive to the scene.
 * @param kind Primitive kind: "Cube", "Plane", "Sphere", "Cylinder", "Cylinder8", "Capsule", "Cone".
 */
int32_t petunia_add_primitive(PetuniaContext* ctx, const char* kind);

/**
 * @brief Undo the last action.
 */
int32_t petunia_undo(PetuniaContext* ctx);

/**
 * @brief Redo the last undone action.
 */
int32_t petunia_redo(PetuniaContext* ctx);

/**
 * @brief Select all vertices/edges/faces in the active asset.
 */
int32_t petunia_select_all(PetuniaContext* ctx);

/**
 * @brief Clear current selection in the active asset.
 */
int32_t petunia_clear_selection(PetuniaContext* ctx);

/**
 * @brief Delete selected geometry.
 */
int32_t petunia_delete_selection(PetuniaContext* ctx);

/**
 * @brief Duplicate selected geometry or active asset.
 */
int32_t petunia_duplicate_selection(PetuniaContext* ctx);

/**
 * @brief Extrude selected faces by a distance along their normals.
 */
int32_t petunia_extrude_selection(PetuniaContext* ctx, float distance);

/**
 * @brief Subdivide selected faces.
 */
int32_t petunia_subdivide_selection(PetuniaContext* ctx);

/**
 * @brief Scale selection uniformly by given factor.
 */
int32_t petunia_scale_selection(PetuniaContext* ctx, float scale);

/* ------------------------------------------------------------------------- */
/* Scene & Selection Queries                                                 */
/* ------------------------------------------------------------------------- */

/**
 * @brief Get total number of 3D assets currently in the project.
 */
int32_t petunia_get_asset_count(PetuniaContext* ctx);

/**
 * @brief Query total vertex and face counts across the whole scene.
 */
int32_t petunia_get_scene_summary(PetuniaContext* ctx, uint32_t* out_total_verts, uint32_t* out_total_faces);

/**
 * @brief Copy name of the currently active asset into buffer.
 */
int32_t petunia_get_active_asset_name(PetuniaContext* ctx, char* buffer, size_t buffer_len);

/**
 * @brief Query vertex and face counts of the currently active asset.
 */
int32_t petunia_get_active_asset_stats(PetuniaContext* ctx, uint32_t* out_verts, uint32_t* out_faces);

/**
 * @brief Activate an asset by its canonical UUID string (e.g. "a1b2c3d4-...").
 */
int32_t petunia_set_active_asset_by_id(PetuniaContext* ctx, const char* uuid_str);

/**
 * @brief Delete an asset by its canonical UUID string.
 */
int32_t petunia_delete_asset_by_id(PetuniaContext* ctx, const char* uuid_str);

/**
 * @brief Query full scene hierarchy as serialized JSON into caller-provided buffer.
 */
int32_t petunia_query_scene_hierarchy_json(PetuniaContext* ctx, char* buffer, size_t buffer_len);

/**
 * @brief Query selection details as serialized JSON into caller-provided buffer.
 */
int32_t petunia_query_selection_details_json(PetuniaContext* ctx, char* buffer, size_t buffer_len);

/**
 * @brief Query active tool status as serialized JSON into caller-provided buffer.
 */
int32_t petunia_query_tool_status_json(PetuniaContext* ctx, char* buffer, size_t buffer_len);

/* ------------------------------------------------------------------------- */
/* Error Reporting                                                           */
/* ------------------------------------------------------------------------- */

/**
 * @brief Copy the last thread-local error message into caller-provided buffer.
 */
int32_t petunia_last_error_message(char* buffer, size_t buffer_len);

#ifdef __cplusplus
}
#endif

#endif /* PETUNIA_H */
