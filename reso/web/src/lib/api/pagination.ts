export interface PagedResponse<T> {
	items: T[];
	total: number | null;
	top: number;
	skip: number;
	has_more: boolean;
	next_offset: number;
}

export interface PagedRequest {
	top: number;
	skip: number;
}
