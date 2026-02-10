import { isHTTPError } from "ky";

export interface ApiError {
	error: string;
	message: string;
}


export function isApiError(e: unknown): e is ApiError {

	if (!(typeof e === 'object' && e !== null)) return false;

	return (
		'message' in e &&
		'error' in e &&
		typeof e.error === 'string' &&
		typeof e.message === 'string'
	);

}

export async function getApiError(e: unknown): Promise<ApiError | undefined> {

	if (!isHTTPError(e)) return;

	const jsonResp = await e.response.json<unknown>();

	if (isApiError(jsonResp)) {
		return jsonResp;
	}

}