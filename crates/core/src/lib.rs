//! Petunia3D core neutro: tipos compartilhados, eventos e contrato de módulo.
//! Features conhecem estas abstrações — nunca umas às outras (§22).

pub mod animate_session;
pub mod brush;
pub mod camera;
pub mod command;
pub mod cutting_session;
pub mod diagnostics;
pub mod docs;
pub mod events;
pub mod handles;
pub mod inference;
pub mod jobs;
pub mod loop_cut;
pub mod mesh_preview;
pub mod modal;
pub mod modal_feedback;
pub mod module;
pub mod picking;
pub mod poly_pen;
pub mod primitive_session;
pub mod project_service;
pub mod proportional;
pub mod queries;
pub mod recent_projects;
pub mod region_push;
pub mod render_revision;
pub mod rig_commands;
pub mod schema_contracts;
pub mod selection;
pub mod shape_builder;
pub mod snap;
pub mod state;
pub mod tool_session;
pub mod transform_projection;
pub mod viewport;
pub mod viewport_query;

pub use animate_session::{
    AnimatePreview, AnimateSession, MotionCatalogEntry, MotionUnavailable, PosePreview,
    PreviewBone, motion_catalog,
};
pub use cutting_session::CutSession;
pub use diagnostics::{DiagnosticCategory, DiagnosticEvent, log_event};
pub use docs::DocsTopic;
pub use handles::{Handle, HandleTable};
pub use jobs::{JobChannel, JobsError, par_finite_sum, par_validate};
pub use petunia_project::{
    ArcLengthTable, AssetSummary, AutosaveConfig, AutosaveService, MAX_GENERATED_VERTICES,
    ModelLibraryQuery, ModelLibraryService, ModelLibrarySort, PathGenerator,
    PathGeneratorDependencies, PathGeneratorDiagnostics, PathGeneratorError,
    PathGeneratorEvaluation, PathGeneratorEvaluationCache, PathGeneratorKind, PathGeneratorQuality,
    PathGeneratorWarning, ProfileError, ProfileResource, ProfileWorkplane, RecoveryInfo,
    SessionLockInfo, SplineError, SplineEvaluationCache, SplineFrame, SplineHandleMode,
    SplineInterpolation, SplinePoint, SplineResource, SplineSample, SplineSnapSettings,
    SurfaceAttachment, SurfaceAttachmentError, SurfaceAttachmentStatus, SurfaceFrame, SurfaceHit,
    SurfaceTriangleHandle, SweepGeneratorParameters, detach_surface_attachment_keep_world,
    evaluate_surface_attachment, project_ray_to_surface, project_ray_to_surface_target,
    reproject_surface_attachment, reproject_surface_attachment_to_target, slide_surface_attachment,
    snap_spline_position, surface_attachment_status,
};
pub use queries::{
    SceneHierarchyDto, SceneObjectDto, SelectionDetailsDto, ToolStatusDto, UvDiagnosticsDto,
};
pub use recent_projects::{RecentProjectEntry, RecentProjects};
pub use render_revision::{FingerprintFlags, SceneFingerprint, fingerprint_scene};
pub use schema_contracts::{
    CONTRACT_SCHEMA_VERSION, CommandIntent, SceneItemContract, command_intent_schema,
    scene_item_schema,
};

pub use command::{
    AddPrimitiveCmd, AddSplinePointCmd, AttachSplinePointCmd, BakeDecalCmd, BakePathGeneratorCmd,
    BevelCmd, BooleanOpCmd, BoxSelectCmd, ClearSelectionCmd, Command, CommandCategory,
    CommandDispatcher, CommandError, CommandMetadata, CommandPaletteItem, ConnectLoopsCmd,
    ConvertSplineToPolylineCmd, CreatePathGeneratorCmd, CreateProfileCmd, CreateSplineCmd,
    CycleSelectionDomainCmd, DeleteAssetCmd, DeleteOrDissolveSelectionCmd, DeletePathGeneratorCmd,
    DeleteProfileCmd, DeleteSelectionCmd, DeleteSplineCmd, DeleteSplinePointCmd,
    DetachSplinePointCmd, DuplicateAssetCmd, DuplicateSelectionCmd, ExportGlbCmd, ExportObjCmd,
    ExtrudeIndividualCmd, ExtrudeSelectedCmd, FlipDiagonalCmd, FlipNormalsCmd, FrameSelectionCmd,
    ImportGltfCmd, ImportObjCmd, InsetFacesCmd, InstantiateAssetCmd, InvertSelectionCmd,
    JoinObjectsCmd, KnifeToolCmd, LoopCutCmd, MergeCenterCmd, MoveSplinePointCmd, NewProjectCmd,
    OpenProjectCmd, PrimitiveKind, PushPullToolCmd, RedoCmd, ReorderAssetCmd,
    ReprojectSplinePointAttachmentCmd, ResetCameraCmd, ReverseSplineCmd, RevolveCmd,
    RotateSplinePointAttachmentCmd, SaveActiveAsAssetCmd, SaveProjectAsCmd, SaveProjectCmd,
    ScaleSelectionCmd, SelectAllCmd, SelectLinkedCmd, SeparateSelectionCmd, SetAssetCollectionCmd,
    SetDecalTransformCmd, SetSelectionDomainCmd, SetShadeSmoothCmd, SetSplineClosedCmd,
    SetSplineHandlesCmd, SlideSplinePointAttachmentCmd, SubdivideSelectionCmd, SymmetrizeCmd,
    ToggleCollectionLockCmd, ToggleCollectionVisibilityCmd, ToggleCommandPaletteCmd, ToggleHelpCmd,
    ToggleLockAssetCmd, ToggleProjectionCmd, ToggleSettingsCmd, ToggleVisibilityAssetCmd,
    ToggleWireframeCmd, ToggleXRayCmd, UndoCmd, UnwrapAutoCmd, UpdateProfileCmd, UpdateSplineCmd,
    UpdateSweepGeneratorCmd, UvPackIslandsCmd, UvProjectFromViewCmd, WeldCmd,
};
pub use project_service::{ProjectService, ProjectServiceError, sanitize_filename};
pub use rig_commands::{
    AddAnimationCmd, AddIkChainCmd, AddMotionCmd, AddRigPresetCmd, ApplyMotionNowCmd,
    AssignRigRoleCmd, AutoRigActiveAssetCmd, ClearRigRoleCmd, DeleteBoneKeyCmd, DuplicateMotionCmd,
    FitBlocked, FitRigToActiveAssetCmd, InferRigRolesCmd, RemoveAnimationCmd, RemoveIkChainCmd,
    RemoveMotionCmd, RemoveSkeletonCmd, RigPresetKind, SetBoneKeyCmd, SetMotionParamCmd,
    SetMotionStyleCmd, UpdateIkChainCmd, UpdateMotionCmd, fit_availability,
};

pub use brush::{
    BRUSH_PX_PER_UNIT, BrushBlend, BrushLock, BrushPreset, BrushPreviewKind, BrushPreviewStyle,
    BrushProjectionMode, BrushSettings, BrushStyle, BrushTip, BrushType, FillScope,
    PaintRestriction, PointStabilizer, StrokeBuffer, StrokeSampler, brush_size_px_from_slider,
    brush_type_from_kind, hash01, kind_from_brush_type,
};
pub use camera::{Camera, Projection, ViewPreset};
pub use events::{AppEvent, EventBus};
pub use inference::{
    DEFAULT_SNAP_RADIUS_PIXELS, ScreenSnapHit, ScreenSnapQuery, SnapAnchor, SnapGrid, SnapKind,
    SnapMask, clamp_snap_radius, snap_screen,
};
pub use modal_feedback::ToolFeedback;
pub use module::{Module, ModuleRegistry};
pub use petunia_mesh::poly_pen::PenPoint;
pub use poly_pen::PolyPenCommandError;
pub use primitive_session::{
    CircleFill, PrimitiveCreationSession, PrimitiveDescriptor, PrimitiveDescriptorExt,
};
pub use proportional::{ProportionalFalloff, ProportionalSettings, calculate_falloff_weight};
pub use region_push::{RegionHit, RegionPlane, RegionPlanes, RegionPushError, region_at_ray};
pub use selection::{SelectMode, Selection, SelectionDomain, Workspace};
pub use shape_builder::{
    PathfinderOp, ShapeEdit, ShapeEditCmd, ShapeEditError, ShapeEditReport, ShapePlane,
};
pub use snap::{
    SnapElement, SnapQuery, SnapResult, SnapSettings, SnapTarget, snap_point, snap_point_to_edges,
    snap_point_to_faces, snap_point_to_grid, snap_point_to_increment, snap_point_to_vertices,
};
pub use state::{
    ASSET_NAME_MAX_LEN, AnnotationItem, AnnotationStroke, AppState, AssetRenameError, DirtyReason,
    DockOrientation, DockSide, DomainState, EditMode, EditorSession, GridSettings, HoverTarget,
    Measurement, MeasurementItem, PROPERTIES_DEFAULT_WIDTH, PROPERTIES_MAX_WIDTH,
    PROPERTIES_MIN_WIDTH, PivotPoint, ProfileState, ProjectState, RefAxis, ReferenceImage,
    RenderResources, RenderStats, SHELL_ASSET_LIBRARY_DEFAULT_HEIGHT,
    SHELL_ASSET_LIBRARY_MAX_HEIGHT, SHELL_ASSET_LIBRARY_MIN_HEIGHT, SceneFilter, SceneObjectState,
    Shading, TOOLBAR_DEFAULT_WIDTH, TextureDirtyRect, TextureUpdate, ToolActivation, ToolState,
    TransformOrientation, UiDensity, UiState, WorkplaneKind, WorkspaceUiMemory, workspace_index,
};
pub use viewport::{
    LogicalRect, PhysicalViewport, unproject_cursor_or_vertex_snap,
    unproject_to_surface_or_cursor_plane,
};
pub use viewport_query::{ViewportQueryBuffer, ViewportQuerySample};

pub type AttachmentValidity = SurfaceAttachmentStatus;

pub use modal::{ModalConstraint, ModalError, ModalKind, ModalOp};
pub use tool_session::{
    DEFAULT_DRAG_THRESHOLD_PX, DRAG_THRESHOLD_RANGE, DragFrame, LastOperation, PressTarget,
    ToolEffect, ToolKey, ToolPhase, ToolSession, drag_value,
};

/// Malha, reexportada para os shells que manipulam geometria sem depender de `petunia_mesh`.
pub use petunia_mesh::Mesh;
/// Ponto de corte de aresta da faca, reexportado para os shells não dependerem
/// de `petunia_mesh` diretamente.
pub use petunia_mesh::knife::EdgePoint as CutEdgePoint;
/// Anel de faces/arestas do loop cut, reexportado pelo mesmo motivo.
pub use petunia_mesh::loop_cut::{LoopCutError, LoopRing};

#[cfg(test)]
mod preview_tests;
